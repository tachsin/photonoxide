//! The static solver that [`crate::thermal`] and [`crate::electro`] share: −∇·(c∇u) = s on a
//! cross-section, by cell-centred finite volumes on a graded tensor-product grid.
//!
//! One operator serves three problems: heat conduction (u the temperature, c the thermal
//! conductivity k, s the heat a heater gives), electrostatics (u the potential, c the
//! permittivity ε₀ε_r, s a charge) and current continuity (u the potential, c the electrical
//! conductivity σ, no source). Everything is per unit length along the guide, which runs out of
//! the plane: a heater's power in W/m, a conductor's charge in C/m, a capacitance in F/m.
//!
//! **The grid** ([`Grid`], [`Axis`], [`Grading`]) is a tensor product of two axes whose nodes
//! fall on every line a [`Features`] holds (the layers' interfaces, the straight edges of the
//! shapes, the electrodes), graded geometrically from the smallest step at the features'
//! corners to the largest far from them, so a window can reach far boundaries cheaply and an
//! electrode's edge is resolved.
//!
//! **The scheme.** Each cell's balance: the flux through each face, c (u_N − u_P) times the
//! face's length over the distance between the centres, equals the source in the cell. A face
//! between two media takes their resistances in series, d_P/c_P + d_N/c_N (the harmonic mean
//! of S. V. Patankar, *Numerical Heat Transfer and Fluid Flow*, Hemisphere (1980), Section
//! 4.2-3; CRC Press reissue 2018, doi:10.1201/9781482234213), which is exact for a layered
//! medium whose interfaces are on the grid's lines: then T is piecewise linear across the
//! layers and the scheme reproduces it to round-off. Cells a curved or sloped boundary cuts
//! take their coefficient from the fill ([`crate::averaging`]): along the interface the
//! fill-weighted mean, across it the harmonic mean (Farjadpour et al.'s diagonal, Opt. Lett.
//! 31, 2972 (2006), doi:10.1364/OL.31.002972), as ε is averaged for the optics. On a graded
//! grid the flux between centres is only first-order accurate locally, but the solution is
//! second order (supraconvergence: T. A. Manteuffel, A. B. White, Math. Comp. 47, 511 (1986),
//! doi:10.1090/S0025-5718-1986-0856700-3); the validation cases measure it.
//!
//! **Held regions.** A conductor at a potential, or a heat sink at a temperature, holds the
//! cells whose centres it contains at its value. A free cell beside one meets it at the
//! boundary's own place along the line between their centres, not at the face (L = c/δ with δ
//! the true distance: G. H. Shortley, R. Weller, J. Appl. Phys. 9, 334 (1938),
//! doi:10.1063/1.1710426), so a curved conductor converges at second order where staircasing
//! would be first. A strip of zero thickness on a grid line (a coplanar line's electrodes)
//! holds the faces it covers. A held region's flux, its charge or the heat it takes, is the sum
//! over the faces it meets: exactly what the free cells' balances leave, so the total balance
//! (sources and held regions against the window's sides) closes to round-off.
//!
//! **Boundaries** on each side of the window ([`Boundary`]): a fixed value, a flux, or a
//! convective film (a coefficient h and an ambient value).
//!
//! **The solve** is photonoxide's own multifrontal L D Lᵀ ([`crate::sparse`]) on the free
//! cells, numbered by nested dissection: the matrix is symmetric positive definite, its
//! factors are reused for every right-hand side (a capacitance matrix's conductors), and the
//! same bits come out on any number of threads.

use std::sync::Arc;

use faer::sparse::{SparseColMat, Triplet};
use num_complex::Complex64 as c64;

use crate::averaging::{Layout, Shares};
use crate::geometry::{Bounds, Point, Region, Shape};
use crate::sparse::{Analysis, Multifrontal};
use crate::stack::{Fill, Structure};
use crate::units::Length;
use crate::{Error, Result};

pub(crate) mod checks;
#[cfg(test)]
mod tests;

/// The most cells an axis may have.
pub const MAX_CELLS: usize = 20_000;

/// Half the width of a band ([`Section::layer`]) across the plane, µm: far beyond any window.
const BAND: f64 = 1e6;

fn invalid(reason: impl Into<String>) -> Error {
    Error::invalid("static problem", reason)
}

/// One axis of a [`Grid`]: its nodes, the cells' edges, in µm, strictly increasing.
#[derive(Clone, Debug, PartialEq)]
pub struct Axis {
    nodes: Vec<f64>,
}

impl Axis {
    /// An axis with these nodes (µm): at least two, finite, strictly increasing.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] otherwise, or with more than [`MAX_CELLS`] cells.
    pub fn new(nodes: Vec<f64>) -> Result<Axis> {
        if nodes.len() < 2 || nodes.len() > MAX_CELLS + 1 {
            return Err(invalid(format!(
                "an axis needs 2 to {} nodes, got {}",
                MAX_CELLS + 1,
                nodes.len()
            )));
        }
        if nodes.iter().any(|v| !v.is_finite()) || nodes.windows(2).any(|w| w[1] <= w[0]) {
            return Err(invalid(
                "an axis's nodes must be finite and strictly increasing",
            ));
        }
        Ok(Axis { nodes })
    }

    /// `cells` equal cells from `a` to `b` (µm).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless a < b, both finite, and 1 ≤ `cells` ≤ [`MAX_CELLS`].
    pub fn uniform(a: f64, b: f64, cells: usize) -> Result<Axis> {
        if !(a.is_finite() && b.is_finite() && a < b) || cells == 0 {
            return Err(invalid(format!(
                "a uniform axis needs a < b and at least one cell, got [{a}, {b}] and {cells}"
            )));
        }
        let h = (b - a) / cells as f64;
        let mut nodes: Vec<f64> = (0..cells).map(|i| a + h * i as f64).collect();
        nodes.push(b);
        Axis::new(nodes)
    }

    /// From `a` to `b` (µm), with a node on every line of `lines` between them, cells no larger
    /// than `grading`'s largest, and no larger than its smallest plus (growth − 1) times the
    /// distance to the nearest point of `fine` (a geometric progression away from each of
    /// them). A step that would leave a sliver before a line is shared with the one before it.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless a < b (finite), or with more than [`MAX_CELLS`] cells.
    pub fn graded(a: f64, b: f64, grading: &Grading, lines: &[f64], fine: &[f64]) -> Result<Axis> {
        if !(a.is_finite() && b.is_finite() && a < b) {
            return Err(invalid(format!(
                "an axis needs finite ends a < b, got [{a}, {b}]"
            )));
        }
        let tiny = 1e-9 * (b - a);
        let fine: Vec<f64> = fine.iter().copied().filter(|v| v.is_finite()).collect();
        let step = |x: f64| -> f64 {
            let near = fine
                .iter()
                .map(|&p| grading.smallest + (grading.growth - 1.0) * (x - p).abs())
                .fold(f64::INFINITY, f64::min);
            near.min(grading.largest)
        };
        // the smallest step over [x, x + h]: the ends', or the smallest where a fine point
        // falls inside (the step function is a minimum of V shapes)
        let least = |x: f64, h: f64| -> f64 {
            let mut s = step(x).min(step(x + h));
            for &p in &fine {
                if p > x && p < x + h {
                    s = s.min(grading.smallest);
                }
            }
            s
        };
        let mut stops: Vec<f64> = lines
            .iter()
            .copied()
            .filter(|&v| v.is_finite() && v > a + tiny && v < b - tiny)
            .collect();
        stops.push(b);
        stops.sort_by(f64::total_cmp);
        stops.dedup_by(|x, y| (*x - *y).abs() <= tiny);
        let mut nodes = vec![a];
        for stop in stops {
            let mut x = *nodes.last().unwrap_or(&a);
            loop {
                let mut h = step(x);
                // shrink until the cell is no larger than the step anywhere in it
                for _ in 0..60 {
                    let s = least(x, h);
                    if s >= h * (1.0 - 1e-12) {
                        break;
                    }
                    h = s;
                }
                if x + h >= stop - tiny {
                    nodes.push(stop);
                    break;
                }
                if stop - (x + h) < 0.5 * step(x + h) {
                    // a sliver would be left: two equal steps to the stop
                    nodes.push(0.5 * (x + stop));
                    nodes.push(stop);
                    break;
                }
                x += h;
                nodes.push(x);
                if nodes.len() > MAX_CELLS + 1 {
                    return Err(invalid(format!(
                        "an axis from {a} to {b} µm would have more than {MAX_CELLS} cells: \
                         the smallest step {} µm is too small for it",
                        grading.smallest
                    )));
                }
            }
        }
        Axis::new(nodes)
    }

    /// The nodes, µm.
    pub fn nodes(&self) -> &[f64] {
        &self.nodes
    }

    /// The number of cells.
    pub fn cells(&self) -> usize {
        self.nodes.len() - 1
    }

    /// Cell `i`'s centre, µm.
    pub fn centre(&self, i: usize) -> f64 {
        0.5 * (self.nodes[i] + self.nodes[i + 1])
    }

    /// Cell `i`'s width, µm.
    pub fn width(&self, i: usize) -> f64 {
        self.nodes[i + 1] - self.nodes[i]
    }

    /// The smallest and largest cells' widths, µm.
    pub fn steps(&self) -> (f64, f64) {
        (0..self.cells())
            .map(|i| self.width(i))
            .fold((f64::INFINITY, 0.0), |(lo, hi), w| (lo.min(w), hi.max(w)))
    }

    /// The first and last nodes, µm.
    pub fn span(&self) -> [f64; 2] {
        [self.nodes[0], self.nodes[self.nodes.len() - 1]]
    }

    /// The cell holding `v` (the nearest end cell outside the axis).
    pub(crate) fn cell_of(&self, v: f64) -> usize {
        self.nodes
            .partition_point(|&n| n <= v)
            .clamp(1, self.nodes.len() - 1)
            - 1
    }
}

/// How a graded axis's cells are sized ([`Axis::graded`]): the largest step, the smallest (at
/// the features' corners), and how much larger each cell may be than its neighbour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grading {
    largest: f64,
    smallest: f64,
    growth: f64,
}

impl Grading {
    /// Cells from `smallest` to `largest` µm, each at most `growth` (≥ 1) times its
    /// neighbour toward the nearest fine point.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless 0 < smallest ≤ largest and growth ≥ 1, all finite.
    pub fn new(largest: f64, smallest: f64, growth: f64) -> Result<Grading> {
        if !(smallest.is_finite()
            && largest.is_finite()
            && growth.is_finite()
            && smallest > 0.0
            && smallest <= largest
            && growth >= 1.0)
        {
            return Err(invalid(format!(
                "a grading needs 0 < smallest ≤ largest and growth ≥ 1, got smallest {smallest}, \
                 largest {largest}, growth {growth}"
            )));
        }
        Ok(Grading {
            largest,
            smallest,
            growth,
        })
    }

    /// Every cell `step` µm or less: no grading.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless `step` is positive and finite.
    pub fn uniform(step: f64) -> Result<Grading> {
        Grading::new(step, step, 1.0)
    }

    /// The same grading with every step divided by `factor`: for a convergence study.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless `factor` is positive and finite.
    pub fn refined(&self, factor: f64) -> Result<Grading> {
        if !(factor.is_finite() && factor > 0.0) {
            return Err(invalid(format!(
                "a grading is refined by a positive factor, got {factor}"
            )));
        }
        Grading::new(self.largest / factor, self.smallest / factor, self.growth)
    }

    /// The largest step, µm.
    pub fn largest(&self) -> f64 {
        self.largest
    }

    /// The smallest step, µm.
    pub fn smallest(&self) -> f64 {
        self.smallest
    }

    /// The largest ratio between neighbouring cells.
    pub fn growth(&self) -> f64 {
        self.growth
    }
}

/// What a fitted [`Grid`] puts nodes on and refines toward: lines along each axis (an
/// interface, a straight edge) and corners (where an electrode's edge or a shape's corner
/// concentrates the field).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Features {
    xs: Vec<f64>,
    ys: Vec<f64>,
    corners: Vec<[f64; 2]>,
}

impl Features {
    /// No features.
    pub fn new() -> Features {
        Features::default()
    }

    /// A vertical line x = `x` (µm) the grid's nodes fall on.
    pub fn line_x(&mut self, x: f64) -> &mut Features {
        self.xs.push(x);
        self
    }

    /// A horizontal line y = `y` (µm).
    pub fn line_y(&mut self, y: f64) -> &mut Features {
        self.ys.push(y);
        self
    }

    /// A corner (µm) the grid refines toward along both axes.
    pub fn corner(&mut self, x: f64, y: f64) -> &mut Features {
        self.corners.push([x, y]);
        self
    }

    /// A shape's straight edges along the axes as lines, and their ends as corners: a
    /// rectangle's four sides and corners, a polygon's axis-aligned edges and its vertices.
    /// Curved shapes add nothing (their cells take the fill).
    pub fn shape(&mut self, shape: &Shape) -> &mut Features {
        match shape {
            Shape::Rect {
                center,
                width,
                height,
            } => {
                let (cx, cy) = (center.x.to_um(), center.y.to_um());
                let (w, h) = (width.to_um() / 2.0, height.to_um() / 2.0);
                for x in [cx - w, cx + w] {
                    self.xs.push(x);
                    for y in [cy - h, cy + h] {
                        self.corners.push([x, y]);
                    }
                }
                self.ys.extend([cy - h, cy + h]);
            }
            Shape::Polygon(p) => {
                let v = p.vertices();
                for (k, a) in v.iter().enumerate() {
                    let b = v[(k + 1) % v.len()];
                    let (ax, ay, bx, by) = (a.x.to_um(), a.y.to_um(), b.x.to_um(), b.y.to_um());
                    if ax == bx {
                        self.xs.push(ax);
                    }
                    if ay == by {
                        self.ys.push(ay);
                    }
                    self.corners.push([ax, ay]);
                }
            }
            _ => {}
        }
        self
    }

    /// A strip's ends as corners and its line (µm): a coplanar line's electrode.
    pub fn strip(&mut self, x: [f64; 2], y: f64) -> &mut Features {
        self.xs.extend(x);
        self.ys.push(y);
        self.corners.extend([[x[0], y], [x[1], y]]);
        self
    }

    /// Adds `other`'s lines and corners.
    pub fn extend(&mut self, other: &Features) -> &mut Features {
        self.xs.extend(&other.xs);
        self.ys.extend(&other.ys);
        self.corners.extend(&other.corners);
        self
    }
}

/// The tensor-product grid a static problem is solved on: cells between the nodes of two
/// axes, x across and y up, in µm.
#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    x: Axis,
    y: Axis,
}

impl Grid {
    /// The grid of the axes `x` and `y`.
    pub fn new(x: Axis, y: Axis) -> Grid {
        Grid { x, y }
    }

    /// The grid over the window `x` × `y` (µm) with a node on every line of `features` and
    /// cells graded by `grading` from their corners, which refine both axes.
    ///
    /// # Errors
    ///
    /// As [`Axis::graded`].
    pub fn fitted(
        x: [f64; 2],
        y: [f64; 2],
        grading: &Grading,
        features: &Features,
    ) -> Result<Grid> {
        let fx: Vec<f64> = features.corners.iter().map(|c| c[0]).collect();
        let fy: Vec<f64> = features.corners.iter().map(|c| c[1]).collect();
        Ok(Grid {
            x: Axis::graded(x[0], x[1], grading, &features.xs, &fx)?,
            y: Axis::graded(y[0], y[1], grading, &features.ys, &fy)?,
        })
    }

    /// Across.
    pub fn x(&self) -> &Axis {
        &self.x
    }

    /// Up.
    pub fn y(&self) -> &Axis {
        &self.y
    }

    /// The number of cells, nx × ny.
    pub fn cells(&self) -> usize {
        self.x.cells() * self.y.cells()
    }

    /// Cell (i, j)'s bounds.
    fn bounds(&self, i: usize, j: usize) -> Bounds {
        Bounds::new(
            Point::um(self.x.nodes[i], self.y.nodes[j]),
            Point::um(self.x.nodes[i + 1], self.y.nodes[j + 1]),
        )
    }

    /// Each cell's area inside `shape` (µm²), for the cells it reaches: (cell, area).
    pub(crate) fn overlap(&self, shape: &Shape) -> Vec<(usize, f64)> {
        let b = shape.bounds();
        let (i0, i1) = (
            self.x.cell_of(b.min.x.to_um()),
            self.x.cell_of(b.max.x.to_um()),
        );
        let (j0, j1) = (
            self.y.cell_of(b.min.y.to_um()),
            self.y.cell_of(b.max.y.to_um()),
        );
        let nx = self.x.cells();
        let mut out = Vec::new();
        for j in j0..=j1 {
            for i in i0..=i1 {
                let a = shape.fill(self.bounds(i, j));
                if a > 0.0 {
                    out.push((j * nx + i, a));
                }
            }
        }
        out
    }

    /// The cells and weights [`Field::at`] interpolates (x, y) from (µm): bilinear between the
    /// four nearest cells' centres, the nearest edge cells' beyond the outermost centres.
    pub(crate) fn bilinear(&self, x: f64, y: f64) -> [(usize, f64); 4] {
        let locate = |a: &Axis, v: f64| -> (usize, usize, f64) {
            let n = a.cells();
            if n == 1 || v <= a.centre(0) {
                return (0, 0, 0.0);
            }
            if v >= a.centre(n - 1) {
                return (n - 1, n - 1, 0.0);
            }
            let mut i = a.cell_of(v);
            if v < a.centre(i) {
                i -= 1;
            }
            let t = (v - a.centre(i)) / (a.centre(i + 1) - a.centre(i));
            (i, i + 1, t)
        };
        let (i0, i1, s) = locate(&self.x, x);
        let (j0, j1, t) = locate(&self.y, y);
        let nx = self.x.cells();
        [
            (j0 * nx + i0, (1.0 - t) * (1.0 - s)),
            (j0 * nx + i1, (1.0 - t) * s),
            (j1 * nx + i0, t * (1.0 - s)),
            (j1 * nx + i1, t * s),
        ]
    }
}

/// A medium's coefficient in the cross-section's frame: diagonal, `xx` across and `yy` up (a
/// thermal conductivity in W/(m·K), a relative permittivity, or an electrical conductivity in
/// S/m, as the problem says).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Tensor {
    /// Across.
    pub xx: f64,
    /// Up.
    pub yy: f64,
}

impl Tensor {
    /// The same along both axes.
    pub fn isotropic(value: f64) -> Tensor {
        Tensor {
            xx: value,
            yy: value,
        }
    }

    /// `xx` across and `yy` up: a crystal whose axes are the section's (x-cut or z-cut lithium
    /// niobate).
    pub fn diagonal(xx: f64, yy: f64) -> Tensor {
        Tensor { xx, yy }
    }

    fn check(&self, medium: &str) -> Result<()> {
        if self.xx.is_finite() && self.yy.is_finite() && self.xx > 0.0 && self.yy > 0.0 {
            Ok(())
        } else {
            Err(invalid(format!(
                "{medium}'s coefficient must be positive and finite, got [{}, {}]",
                self.xx, self.yy
            )))
        }
    }
}

/// A side of the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    /// x = the window's start.
    Left,
    /// x = its end.
    Right,
    /// y = its start.
    Bottom,
    /// y = its end.
    Top,
}

impl Side {
    /// The four sides, in the order [`Field::outflow`] reports them.
    pub const ALL: [Side; 4] = [Side::Left, Side::Right, Side::Bottom, Side::Top];

    fn index(self) -> usize {
        match self {
            Side::Left => 0,
            Side::Right => 1,
            Side::Bottom => 2,
            Side::Top => 3,
        }
    }

    /// The side's name, as a job writes it.
    pub fn name(self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Right => "right",
            Side::Bottom => "bottom",
            Side::Top => "top",
        }
    }
}

/// What holds a side of the window.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Boundary {
    /// A fixed value: a temperature (K or °C, as the problem's values are) or a potential (V).
    Fixed(f64),
    /// A flux into the window per unit area: W/m² of heat, or C/m² of surface charge. Zero is
    /// an insulated side or a mirror plane.
    Flux(f64),
    /// A convective film: the flux into the window is `coefficient` (W/(m²·K)) times (ambient −
    /// T) at the side.
    Convective {
        /// The film coefficient h, W/(m²·K).
        coefficient: f64,
        /// The ambient value.
        ambient: f64,
    },
}

impl Boundary {
    /// No flux: an insulated side, or a mirror plane.
    pub const INSULATED: Boundary = Boundary::Flux(0.0);

    fn check(&self, side: Side) -> Result<()> {
        let ok = match *self {
            Boundary::Fixed(v) | Boundary::Flux(v) => v.is_finite(),
            Boundary::Convective {
                coefficient,
                ambient,
            } => coefficient.is_finite() && coefficient > 0.0 && ambient.is_finite(),
        };
        if ok {
            Ok(())
        } else {
            Err(invalid(format!(
                "the {} side's boundary must be finite (a convective coefficient positive), got \
                 {self:?}",
                side.name()
            )))
        }
    }

    fn holds(&self) -> bool {
        !matches!(self, Boundary::Flux(_))
    }
}

/// Where a cross-section is cut from a [`Structure`]: the vertical plane x = `x` (light along
/// x: the section's x is the structure's y), or y = `y` (light along y: the section's x is the
/// structure's x). The section's y is the structure's height z, 0 at the bottom of the first
/// layer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cut {
    /// The plane x = this (µm).
    X(f64),
    /// The plane y = this (µm).
    Y(f64),
}

impl Cut {
    /// The structure's point at the section's x = `u`.
    fn point(self, u: f64) -> Point {
        match self {
            Cut::X(x) => Point::um(x, u),
            Cut::Y(y) => Point::um(u, y),
        }
    }

    /// The direction across the cut in the structure's plane.
    fn across(self) -> [f64; 2] {
        match self {
            Cut::X(_) => [0.0, 1.0],
            Cut::Y(_) => [1.0, 0.0],
        }
    }
}

/// The key of a section's layout where what lies beneath (a structure's cut) shows.
const BENEATH: usize = usize::MAX;

/// What fills a cross-section: shapes drawn with media (by name), later ones on top, over a
/// background medium or over a [`Structure`]'s cut. A problem gives each medium its
/// coefficient ([`Tensor`]).
#[derive(Clone)]
pub struct Section {
    layout: Layout<usize>,
    media: Vec<String>,
    shapes: Vec<Shape>,
    cut: Option<(Structure, Cut)>,
    features: Features,
}

impl std::fmt::Debug for Section {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Section")
            .field("media", &self.media)
            .field("shapes", &self.shapes.len())
            .field("cut", &self.cut.as_ref().map(|c| c.1))
            .finish()
    }
}

impl Section {
    /// A plane of `background`, nothing drawn on it.
    pub fn new(background: impl Into<String>) -> Section {
        Section {
            layout: Layout::new(0),
            media: vec![background.into()],
            shapes: Vec::new(),
            cut: None,
            features: Features::new(),
        }
    }

    /// The cross-section of `structure` at `cut`: the substrate, the layers with their shapes,
    /// etches and sloped walls, and the cladding, each cell filled as the full-vector mode
    /// solver fills it ([`Structure::cut_cell`]: its material, or a sloped wall's shares), and
    /// a body's catalogue material at the cell's centre. Each material is a medium named by
    /// [`crate::material::Material::name`]. Its [`Section::features`] are the layers'
    /// interfaces, the etches' floors and the shapes' edges along the cut, each wall's foot too.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if a body in the structure has a graded medium, which has no
    /// material to give a coefficient.
    pub fn cut(structure: &Structure, cut: Cut) -> Result<Section> {
        if structure
            .bodies()
            .iter()
            .any(|(_, m)| matches!(m, crate::stack::Medium::Graded(_)))
        {
            return Err(invalid(
                "a body of a graded medium has no material whose coefficient a static problem \
                 could take",
            ));
        }
        let mut features = Features::new();
        let stack = structure.stack();
        let mut top = 0.0;
        features.line_y(0.0);
        for layer in stack.layers() {
            let thickness = layer.thickness.to_um();
            top += thickness;
            features.line_y(top);
            for (shape, etch) in structure
                .shapes(&layer.name)
                .iter()
                .zip(structure.etches(&layer.name))
            {
                let depth = etch.depth.map_or(thickness, Length::to_um);
                if etch.depth.is_some() {
                    features.line_y(top - depth);
                }
                let run = etch.run();
                for (d, z) in [(0.0, top), (depth, top - depth)] {
                    if d > 0.0 && run == 0.0 {
                        continue;
                    }
                    for u in crossings(shape, cut, d * run) {
                        features.line_x(u);
                        features.corner(u, z);
                    }
                }
            }
        }
        let mut media: Vec<String> = Vec::new();
        let mut name = |m: &str| {
            if !media.iter().any(|n| n == m) {
                media.push(m.to_owned());
            }
        };
        name(stack.substrate().name());
        name(stack.cladding().name());
        for layer in stack.layers() {
            name(layer.material.name());
            name(layer.background.name());
        }
        for (_, m) in structure.bodies() {
            if let crate::stack::Medium::Material(m) = m {
                name(m.name());
            }
        }
        Ok(Section {
            layout: Layout::new(BENEATH),
            media,
            shapes: Vec::new(),
            cut: Some((structure.clone(), cut)),
            features,
        })
    }

    /// Draws `shape` filled with `medium` over what is there.
    pub fn draw(&mut self, shape: Shape, medium: &str) -> &mut Section {
        let key = self.key(medium);
        self.features.shape(&shape);
        self.layout.push(shape.clone(), key);
        self.shapes.push(shape);
        self
    }

    /// Draws a band of `medium` across the whole plane from `bottom` to `top` (µm): a layer.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless bottom < top, both finite.
    pub fn layer(&mut self, bottom: f64, top: f64, medium: &str) -> Result<&mut Section> {
        if !(bottom.is_finite() && top.is_finite() && bottom < top) {
            return Err(invalid(format!(
                "a layer runs from a lower to a higher y, got [{bottom}, {top}]"
            )));
        }
        let shape = Shape::rect(
            Point::um(0.0, 0.5 * (bottom + top)),
            Length::um(2.0 * BAND),
            Length::um(top - bottom),
        )?;
        let key = self.key(medium);
        self.features.line_y(bottom).line_y(top);
        self.layout.push(shape.clone(), key);
        self.shapes.push(shape);
        Ok(self)
    }

    fn key(&mut self, medium: &str) -> usize {
        match self.media.iter().position(|m| m == medium) {
            Some(k) => k,
            None => {
                self.media.push(medium.to_owned());
                self.media.len() - 1
            }
        }
    }

    /// The media's names: the background (or a cut's materials) and every medium drawn.
    pub fn media(&self) -> &[String] {
        &self.media
    }

    /// The lines and corners a fitted [`Grid`] should follow: the drawn shapes' straight
    /// edges, and a cut's interfaces and walls.
    pub fn features(&self) -> &Features {
        &self.features
    }

    /// The medium at (x, y), µm.
    pub fn medium_at(&self, x: f64, y: f64) -> &str {
        let p = Point::um(x, y);
        match self.layout.at(p) {
            BENEATH => match &self.cut {
                Some((s, cut)) => s.material_at(cut.point(x), Length::um(y)).name(),
                None => &self.media[0],
            },
            k => &self.media[k],
        }
    }

    /// What fills `cell`: each medium's share and the interface's normal.
    fn shares(&self, cell: Bounds) -> Shares<usize> {
        let drawn = self.layout.shares(cell);
        let Some((structure, cut)) = &self.cut else {
            return drawn;
        };
        if !drawn.parts.iter().any(|(k, _)| *k == BENEATH) {
            return drawn;
        }
        let (x0, y0, x1, y1) = (
            cell.min.x.to_um(),
            cell.min.y.to_um(),
            cell.max.x.to_um(),
            cell.max.y.to_um(),
        );
        let centre = cut.point(0.5 * (x0 + x1));
        let fill = structure.cut_cell(centre, 0.5 * (y0 + y1), cut.across(), [x1 - x0, y1 - y0]);
        let index = |m: &str| self.media.iter().position(|n| n == m).unwrap_or(0);
        let (under, normal): (Vec<(usize, f64)>, Option<[f64; 2]>) = match fill {
            Fill::Uniform(m) => (vec![(index(m.name()), 1.0)], None),
            Fill::Wall {
                inside,
                outside,
                fraction,
                normal,
            } => (
                vec![
                    (index(inside.name()), fraction),
                    (index(outside.name()), 1.0 - fraction),
                ],
                Some(normal),
            ),
        };
        let mut parts: Vec<(usize, f64)> = Vec::new();
        let mut add = |k: usize, f: f64| {
            if f <= 0.0 {
                return;
            }
            match parts.iter_mut().find(|(m, _)| *m == k) {
                Some(p) => p.1 += f,
                None => parts.push((k, f)),
            }
        };
        for &(k, f) in &drawn.parts {
            if k == BENEATH {
                for &(m, g) in &under {
                    add(m, f * g);
                }
            } else {
                add(k, f);
            }
        }
        let normal = drawn.normal.or(normal);
        Shares { parts, normal }
    }

    /// Each cell's coefficient on `grid`, from each medium's by `coefficient`: a cell of one
    /// medium has its own, a cell an interface crosses the fill-weighted mean along the
    /// interface and the harmonic mean across it, per axis.
    pub(crate) fn coefficients(
        &self,
        grid: &Grid,
        coefficient: impl Fn(&str) -> Option<Tensor>,
        what: &str,
    ) -> Result<Vec<[f64; 2]>> {
        let mut table = Vec::with_capacity(self.media.len());
        for m in &self.media {
            table.push(match coefficient(m) {
                Some(t) => {
                    t.check(m)?;
                    Some(t)
                }
                None => None,
            });
        }
        let (nx, ny) = (grid.x.cells(), grid.y.cells());
        let mut out = Vec::with_capacity(nx * ny);
        for j in 0..ny {
            for i in 0..nx {
                let shares = self.shares(grid.bounds(i, j));
                let mut values = Vec::with_capacity(shares.parts.len());
                for &(k, f) in &shares.parts {
                    let t = table.get(k).copied().flatten().ok_or_else(|| {
                        invalid(format!(
                            "the medium {} has no {what}",
                            self.media.get(k).map_or("?", String::as_str)
                        ))
                    })?;
                    values.push((t, f));
                }
                out.push(average(&values, shares.normal));
            }
        }
        Ok(out)
    }

    /// Each cell's volume mean of a scalar its media have, by their shares: a heat capacity
    /// per volume, which adds as the media's volumes do (no direction to average along).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if a medium has none (`what` names it), or one that isn't
    /// finite and positive.
    pub(crate) fn means(
        &self,
        grid: &Grid,
        value: impl Fn(&str) -> Option<f64>,
        what: &str,
    ) -> Result<Vec<f64>> {
        let mut table = Vec::with_capacity(self.media.len());
        for m in &self.media {
            table.push(match value(m) {
                Some(v) if v.is_finite() && v > 0.0 => Some(v),
                Some(v) => {
                    return Err(invalid(format!(
                        "the medium {m}'s {what} must be finite and positive, got {v}"
                    )));
                }
                None => None,
            });
        }
        let (nx, ny) = (grid.x.cells(), grid.y.cells());
        let mut out = Vec::with_capacity(nx * ny);
        for j in 0..ny {
            for i in 0..nx {
                let shares = self.shares(grid.bounds(i, j));
                let (mut sum, mut total) = (0.0, 0.0);
                for &(k, f) in &shares.parts {
                    let v = table.get(k).copied().flatten().ok_or_else(|| {
                        invalid(format!(
                            "the medium {} has no {what}",
                            self.media.get(k).map_or("?", String::as_str)
                        ))
                    })?;
                    sum += v * f;
                    total += f;
                }
                out.push(if total > 0.0 { sum / total } else { 0.0 });
            }
        }
        Ok(out)
    }

    /// Each cell's media on `grid`: (the medium's index in [`Section::media`], its share of
    /// the cell), the shares summing to 1.
    pub(crate) fn parts(&self, grid: &Grid) -> Vec<Vec<(usize, f64)>> {
        let (nx, ny) = (grid.x.cells(), grid.y.cells());
        let mut out = Vec::with_capacity(nx * ny);
        for j in 0..ny {
            for i in 0..nx {
                let shares = self.shares(grid.bounds(i, j));
                let total: f64 = shares.parts.iter().map(|p| p.1).sum();
                out.push(if total > 0.0 {
                    shares.parts.iter().map(|&(k, f)| (k, f / total)).collect()
                } else {
                    Vec::new()
                });
            }
        }
        out
    }
}

/// A cell's coefficient from its media's (with their shares) and the interface's normal: per
/// axis, 1 / (n_a² / harmonic + (1 − n_a²) / mean); one medium's own, to the bit.
fn average(values: &[(Tensor, f64)], normal: Option<[f64; 2]>) -> [f64; 2] {
    if let [(t, _)] = values {
        return [t.xx, t.yy];
    }
    let total: f64 = values.iter().map(|v| v.1).sum();
    let axis = |pick: fn(&Tensor) -> f64, n: f64| -> f64 {
        let mean: f64 = values.iter().map(|(t, f)| pick(t) * f).sum::<f64>() / total;
        let harmonic = total / values.iter().map(|(t, f)| f / pick(t)).sum::<f64>();
        match normal {
            Some(_) => 1.0 / (n * n / harmonic + (1.0 - n * n) / mean),
            None => mean,
        }
    };
    let [nx, ny] = normal.unwrap_or([0.0, 0.0]);
    [axis(|t| t.xx, nx), axis(|t| t.yy, ny)]
}

/// Where `shape`'s outline, grown by `reach` µm (a sloped wall's foot), crosses `cut`'s line:
/// the section's x of each crossing, ascending.
fn crossings(shape: &Shape, cut: Cut, reach: f64) -> Vec<f64> {
    let b = shape.bounds();
    let (lo, hi, off, olo, ohi) = match cut {
        Cut::X(x) => (
            b.min.y.to_um(),
            b.max.y.to_um(),
            x,
            b.min.x.to_um(),
            b.max.x.to_um(),
        ),
        Cut::Y(y) => (
            b.min.x.to_um(),
            b.max.x.to_um(),
            y,
            b.min.y.to_um(),
            b.max.y.to_um(),
        ),
    };
    if off < olo - reach || off > ohi + reach {
        return Vec::new();
    }
    let (lo, hi) = (lo - reach - 1e-9, hi + reach + 1e-9);
    let inside = |u: f64| -> bool {
        let p = cut.point(u);
        if reach == 0.0 {
            shape.contains(p)
        } else {
            shape.distance(p).signed.to_um() <= reach
        }
    };
    const SAMPLES: usize = 2048;
    let mut out = Vec::new();
    let mut prev = (lo, inside(lo));
    for k in 1..=SAMPLES {
        let u = lo + (hi - lo) * k as f64 / SAMPLES as f64;
        let now = inside(u);
        if now != prev.1 {
            let (mut a, mut c) = (prev.0, u);
            for _ in 0..80 {
                let m = 0.5 * (a + c);
                if inside(m) == prev.1 {
                    a = m;
                } else {
                    c = m;
                }
                if c - a <= 1e-13 * (1.0 + a.abs()) {
                    break;
                }
            }
            out.push(0.5 * (a + c));
        }
        prev = (u, now);
    }
    out
}

/// A region a problem holds at a value: the cells whose centres a shape contains, or the faces
/// a strip of zero thickness covers on a horizontal grid line.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Hold {
    Shape(Shape),
    Strip { x: [f64; 2], y: f64 },
}

/// The discrete problem: a grid, each cell's coefficient (in SI units, per axis), the sides'
/// boundaries, the held regions with their values, and each cell's source (per unit length).
pub(crate) struct Problem<'a> {
    pub grid: &'a Grid,
    pub coefficients: Vec<[f64; 2]>,
    pub sides: [Boundary; 4],
    pub holds: Vec<Hold>,
    /// A fixed side's value along it, in place of its constant: (side, the face's centre along
    /// the side, µm) → value. For checks against closed forms on a finite window.
    pub profile: Option<&'a dyn Fn(Side, f64) -> f64>,
}

/// What a free cell's face meets, outside the free cells: a held region, or a side (with the
/// fixed value at that face).
#[derive(Clone, Copy, Debug)]
enum Target {
    Hold(usize),
    Side(usize, f64),
}

/// One of a free cell's four faces: the cell beyond it (if the window goes on), its length,
/// the cell's half width across it, the axis it is normal to, the window's side it lies on if
/// nothing is beyond, and the strip that holds it, if one does.
struct Face {
    beyond: Option<(usize, usize)>,
    length: f64,
    half: f64,
    axis: usize,
    side: usize,
    strip: Option<usize>,
}

/// The factorized problem, ready for any values of the held regions and any sources.
pub(crate) struct Discrete {
    grid: Grid,
    /// each cell's unknown, or the held region holding it
    cell: Vec<Place>,
    unknowns: usize,
    /// a free cell's link to a held region or a side: (unknown, conductance, target)
    links: Vec<(usize, f64, Target)>,
    /// fluxes in through Flux sides: (unknown, W/m or C/m, side)
    inflows: Vec<(usize, f64, usize)>,
    holds: usize,
    /// the faces between free cells: (a, b, conductance), each once
    faces: Vec<(usize, usize, f64)>,
    /// each face's share of its resistance on a's side, (d_a/c_a) / (d_a/c_a + d_b/c_b)
    splits: Vec<f64>,
    /// the face of its cell each link crosses (0 west, 1 east, 2 south, 3 north)
    link_faces: Vec<u8>,
    /// each cell's coefficient across and up
    coefficients: Vec<[f64; 2]>,
    /// the free cells' ordering and symbolic factorization, for this operator and any shift of
    /// its diagonal ([`Discrete::shifted`])
    analysis: Option<Arc<Analysis>>,
    factors: Option<Multifrontal>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Place {
    Free(usize),
    Held(usize),
}

/// The distance from `from` toward `to` (µm) to where `shape` begins, by bisection; `None`
/// if `to` isn't in it.
fn reach(shape: &Shape, from: [f64; 2], to: [f64; 2]) -> Option<f64> {
    let at = |t: f64| {
        Point::um(
            from[0] + t * (to[0] - from[0]),
            from[1] + t * (to[1] - from[1]),
        )
    };
    if !shape.contains(at(1.0)) {
        return None;
    }
    let (mut a, mut b) = (0.0f64, 1.0f64);
    for _ in 0..60 {
        let m = 0.5 * (a + b);
        if shape.contains(at(m)) {
            b = m;
        } else {
            a = m;
        }
    }
    let length = (to[0] - from[0]).hypot(to[1] - from[1]);
    Some(0.5 * (a + b) * length)
}

impl Discrete {
    /// Assembles and factorizes `problem`.
    pub(crate) fn new(problem: &Problem<'_>) -> Result<Discrete> {
        Discrete::build(problem, true)
    }

    /// Assembles `problem` without factorizing it: its fluxes for a field already solved.
    pub(crate) fn assembled(problem: &Problem<'_>) -> Result<Discrete> {
        Discrete::build(problem, false)
    }

    fn build(problem: &Problem<'_>, factorize: bool) -> Result<Discrete> {
        let grid = problem.grid;
        let (nx, ny) = (grid.x.cells(), grid.y.cells());
        for (side, b) in Side::ALL.iter().zip(&problem.sides) {
            b.check(*side)?;
        }
        // the held cells: the last region whose shape contains the centre
        let mut cell = vec![Place::Free(0); nx * ny];
        for j in 0..ny {
            for i in 0..nx {
                let p = Point::um(grid.x.centre(i), grid.y.centre(j));
                for (k, h) in problem.holds.iter().enumerate() {
                    if let Hold::Shape(s) = h
                        && s.contains(p)
                    {
                        cell[j * nx + i] = Place::Held(k);
                    }
                }
            }
        }
        let mut unknowns = 0;
        for c in &mut cell {
            if let Place::Free(u) = c {
                *u = unknowns;
                unknowns += 1;
            }
        }
        // the strips' faces: a horizontal face at node j under cell (i, j) and over (i, j - 1)
        let tiny = 1e-9 * (1.0 + grid.y.span()[1].abs().max(grid.y.span()[0].abs()));
        let mut strip_at = vec![None::<usize>; nx * (ny + 1)];
        for (k, h) in problem.holds.iter().enumerate() {
            if let Hold::Strip { x, y } = h {
                let Some(j) = grid.y.nodes.iter().position(|&v| (v - y).abs() <= tiny) else {
                    return Err(invalid(format!(
                        "a strip at y = {y} µm must lie on one of the grid's lines (fit the grid \
                         to it)"
                    )));
                };
                let mut any = false;
                for i in 0..nx {
                    let c = grid.x.centre(i);
                    if c > x[0] && c < x[1] {
                        strip_at[j * nx + i] = Some(k);
                        any = true;
                    }
                }
                if !any {
                    return Err(invalid(format!(
                        "a strip from {} to {} µm covers no face of the grid",
                        x[0], x[1]
                    )));
                }
            }
        }
        let mut links = Vec::new();
        let mut inflows = Vec::new();
        let mut faces = Vec::new();
        let mut splits = Vec::new();
        let mut link_faces = Vec::new();
        let coef = &problem.coefficients;
        let centre = |i: usize, j: usize| [grid.x.centre(i), grid.y.centre(j)];
        for j in 0..ny {
            for i in 0..nx {
                let c = j * nx + i;
                let Place::Free(u) = cell[c] else { continue };
                let (hx, hy) = (grid.x.width(i), grid.y.width(j));
                let around = [
                    Face {
                        beyond: (i > 0).then(|| (i - 1, j)),
                        length: hy,
                        half: hx / 2.0,
                        axis: 0,
                        side: 0,
                        strip: None,
                    },
                    Face {
                        beyond: (i + 1 < nx).then(|| (i + 1, j)),
                        length: hy,
                        half: hx / 2.0,
                        axis: 0,
                        side: 1,
                        strip: None,
                    },
                    Face {
                        beyond: (j > 0).then(|| (i, j - 1)),
                        length: hx,
                        half: hy / 2.0,
                        axis: 1,
                        side: 2,
                        strip: strip_at[j * nx + i],
                    },
                    Face {
                        beyond: (j + 1 < ny).then(|| (i, j + 1)),
                        length: hx,
                        half: hy / 2.0,
                        axis: 1,
                        side: 3,
                        strip: strip_at[(j + 1) * nx + i],
                    },
                ];
                for Face {
                    beyond: neighbour,
                    length,
                    half,
                    axis,
                    side,
                    strip,
                } in around
                {
                    let own = coef[c][axis];
                    let before = links.len();
                    if let Some(k) = strip {
                        links.push((u, length * own / half, Target::Hold(k)));
                        link_faces.push(side as u8);
                        continue;
                    }
                    match neighbour {
                        None => match problem.sides[side] {
                            Boundary::Fixed(v) => {
                                let v = match problem.profile {
                                    Some(f) => {
                                        let along = if axis == 0 {
                                            grid.y.centre(j)
                                        } else {
                                            grid.x.centre(i)
                                        };
                                        f(Side::ALL[side], along)
                                    }
                                    None => v,
                                };
                                links.push((u, length * own / half, Target::Side(side, v)));
                            }
                            Boundary::Convective {
                                coefficient,
                                ambient,
                            } => {
                                let g = length / (half / own + 1e6 / coefficient);
                                links.push((u, g, Target::Side(side, ambient)));
                            }
                            Boundary::Flux(q) => inflows.push((u, q * length * 1e-6, side)),
                        },
                        Some((a, b)) => {
                            let n = b * nx + a;
                            match cell[n] {
                                Place::Free(v) => {
                                    if v > u {
                                        let far = 0.5
                                            * if axis == 0 {
                                                grid.x.width(a)
                                            } else {
                                                grid.y.width(b)
                                            };
                                        let (near, beyond) = (half / own, far / coef[n][axis]);
                                        let g = length / (near + beyond);
                                        faces.push((u, v, g));
                                        splits.push(near / (near + beyond));
                                    }
                                }
                                Place::Held(k) => {
                                    let Hold::Shape(s) = &problem.holds[k] else {
                                        unreachable!("only a shape holds cells")
                                    };
                                    let d = reach(s, centre(i, j), centre(a, b))
                                        .unwrap_or(half)
                                        .max(1e-6 * half);
                                    let d = if (d - half).abs() <= 1e-9 * half {
                                        half
                                    } else {
                                        d
                                    };
                                    links.push((u, length * own / d, Target::Hold(k)));
                                }
                            }
                        }
                    }
                    link_faces.resize(links.len(), side as u8);
                    debug_assert!(links.len() <= before + 1);
                }
            }
        }
        let held = links.iter().any(|l| matches!(l.2, Target::Hold(_)))
            || problem.sides.iter().any(Boundary::holds);
        if unknowns > 0 && !held {
            return Err(invalid(
                "nothing holds the solution's level: a static problem needs a fixed or \
                 convective side, a held region (a sink, a conductor, a contact) or a strip",
            ));
        }
        let mut d = Discrete {
            grid: grid.clone(),
            cell,
            unknowns,
            links,
            inflows,
            holds: problem.holds.len(),
            faces,
            splits,
            link_faces,
            coefficients: problem.coefficients.clone(),
            analysis: None,
            factors: None,
        };
        if unknowns > 0 && factorize {
            d.factorize()?;
        }
        Ok(d)
    }

    /// The flux density −c∇u at each cell's centre (across and up; zero in held cells), from
    /// the fluxes through its faces: across, the mean of what crosses its west and east faces
    /// per unit of their length, and up likewise. Each face's flux is the scheme's own (the
    /// coefficients in series, a conductor met where it lies), so the normal component is
    /// continuous across an interface on the grid's lines, as D·n is, and the field
    /// −∇u = (flux density) / c is each medium's own on its side: a central difference of u
    /// across the interface would mix the two.
    pub(crate) fn flux_density(&self, field: &Field) -> Vec<[f64; 2]> {
        let (nx, ny) = (self.grid.x.cells(), self.grid.y.cells());
        let of = self.free_cells();
        // each cell's outflow through its west, east, south and north faces, per unit length
        let mut out = vec![[0.0f64; 4]; self.cell.len()];
        for &(a, b, g) in &self.faces {
            let (ca, cb) = (of[a].min(of[b]), of[a].max(of[b]));
            let q = g * (field.values[ca] - field.values[cb]);
            // cb is ca's east neighbour or its north one
            if cb == ca + 1 && ca % nx + 1 < nx && cb / nx == ca / nx {
                out[ca][1] += q;
                out[cb][0] -= q;
            } else {
                out[ca][3] += q;
                out[cb][2] -= q;
            }
        }
        for (&(u, g, t), &f) in self.links.iter().zip(&self.link_faces) {
            let c = of[u];
            let other = match t {
                Target::Hold(k) => field.levels[k],
                Target::Side(_, v) => v,
            };
            out[c][f as usize] += g * (field.values[c] - other);
        }
        for &(u, q, s) in &self.inflows {
            out[of[u]][s] -= q;
        }
        let mut density = vec![[0.0; 2]; self.cell.len()];
        for j in 0..ny {
            for i in 0..nx {
                let c = j * nx + i;
                if let Place::Free(_) = self.cell[c] {
                    let (hx, hy) = (self.grid.x.width(i) * 1e-6, self.grid.y.width(j) * 1e-6);
                    let o = out[c];
                    density[c] = [(o[1] - o[0]) / (2.0 * hy), (o[3] - o[2]) / (2.0 * hx)];
                }
            }
        }
        density
    }

    /// Each cell's coefficient, across and up.
    pub(crate) fn coefficients(&self) -> &[[f64; 2]] {
        &self.coefficients
    }

    /// Whether cell `c` is held by a region.
    pub(crate) fn is_held(&self, c: usize) -> bool {
        matches!(self.cell[c], Place::Held(_))
    }

    /// The free cells' operator K, its diagonal plus `shift` where one is given.
    fn matrix(&self, shift: Option<&[c64]>) -> Result<SparseColMat<usize, c64>> {
        let n = self.unknowns;
        let mut diagonal = vec![0.0f64; n];
        for &(u, g, _) in &self.links {
            diagonal[u] += g;
        }
        let mut triplets = Vec::with_capacity(n + 2 * self.faces.len());
        for &(a, b, g) in &self.faces {
            diagonal[a] += g;
            diagonal[b] += g;
            triplets.push(Triplet::new(a, b, c64::new(-g, 0.0)));
            triplets.push(Triplet::new(b, a, c64::new(-g, 0.0)));
        }
        for (u, &d) in diagonal.iter().enumerate() {
            let s = shift.map_or(c64::new(0.0, 0.0), |s| s[u]);
            triplets.push(Triplet::new(u, u, c64::new(d, 0.0) + s));
        }
        SparseColMat::<usize, c64>::try_new_from_triplets(n, n, &triplets)
            .map_err(|e| invalid(format!("the matrix can't be assembled: {e:?}")))
    }

    fn factorize(&mut self) -> Result<()> {
        let n = self.unknowns;
        let a = self.matrix(None)?;
        let nx = self.grid.x.cells();
        let mut positions = vec![[0.0; 3]; n];
        for (c, place) in self.cell.iter().enumerate() {
            if let Place::Free(u) = *place {
                positions[u] = [self.grid.x.centre(c % nx), self.grid.y.centre(c / nx), 0.0];
            }
        }
        let analysis = match Analysis::new_symmetric(a.as_ref(), Some(&positions))? {
            Some(an) => an,
            None => Analysis::new(a.as_ref(), Some(&positions))?,
        };
        let analysis = Arc::new(analysis);
        self.factors = Some(Multifrontal::new(analysis.clone(), a.as_ref())?);
        self.analysis = Some(analysis);
        Ok(())
    }

    /// The free unknowns.
    pub(crate) fn unknowns(&self) -> usize {
        self.unknowns
    }

    /// Each free unknown's cell, in the unknowns' order.
    pub(crate) fn free_cells(&self) -> Vec<usize> {
        let mut of = vec![0usize; self.unknowns];
        for (c, place) in self.cell.iter().enumerate() {
            if let Place::Free(u) = *place {
                of[u] = c;
            }
        }
        of
    }

    /// K + diag(`shift`) factorized on this problem's own analysis (its ordering and symbolic
    /// factorization): complex symmetric, for an implicit time step (a real shift) or a
    /// frequency (an imaginary one). `None` without free cells.
    ///
    /// # Errors
    ///
    /// The factorization's, for a shift that makes it singular.
    pub(crate) fn shifted(&self, shift: &[c64]) -> Result<Option<Multifrontal>> {
        let Some(analysis) = &self.analysis else {
            return Ok(None);
        };
        let a = self.matrix(Some(shift))?;
        Ok(Some(Multifrontal::new(analysis.clone(), a.as_ref())?))
    }

    /// K x over the free unknowns.
    pub(crate) fn apply(&self, x: &[f64]) -> Vec<f64> {
        let mut y = vec![0.0; self.unknowns];
        for &(u, g, _) in &self.links {
            y[u] += g * x[u];
        }
        for &(a, b, g) in &self.faces {
            let d = g * (x[a] - x[b]);
            y[a] += d;
            y[b] -= d;
        }
        y
    }

    /// The free unknowns' right-hand side for held regions at `values` and the cells'
    /// `sources`, and the sources' total; with `level` false, the sources alone (no held
    /// region's, side's or flux's part: a fluctuation about a steady state).
    pub(crate) fn load(&self, values: &[f64], sources: &[f64], level: bool) -> (Vec<f64>, f64) {
        let value = |t: Target| -> f64 {
            match t {
                Target::Hold(k) => values[k],
                Target::Side(_, v) => v,
            }
        };
        let mut b = vec![0.0; self.unknowns];
        let mut total = 0.0;
        for (c, place) in self.cell.iter().enumerate() {
            if let Place::Free(u) = *place
                && let Some(&s) = sources.get(c)
            {
                b[u] += s;
                total += s;
            }
        }
        if level {
            for &(u, q, _) in &self.inflows {
                b[u] += q;
            }
            for &(u, g, t) in &self.links {
                b[u] += g * value(t);
            }
        }
        (b, total)
    }

    /// The field whose free cells hold `free`, its held regions at `values`, `source` its
    /// sources' total.
    pub(crate) fn field(&self, free: &[f64], values: &[f64], source: f64) -> Field {
        let mut cells = vec![0.0; self.cell.len()];
        for (c, place) in self.cell.iter().enumerate() {
            cells[c] = match *place {
                Place::Free(u) => free[u],
                Place::Held(k) => values[k],
            };
        }
        let mut held = vec![0.0; self.holds];
        let mut outflow = [0.0; 4];
        for &(u, g, t) in &self.links {
            match t {
                Target::Hold(k) => held[k] += g * (values[k] - free[u]),
                Target::Side(s, v) => outflow[s] += g * (free[u] - v),
            }
        }
        for &(_, q, s) in &self.inflows {
            outflow[s] -= q;
        }
        Field {
            grid: self.grid.clone(),
            values: cells,
            levels: values.to_vec(),
            held,
            outflow,
            source,
        }
    }

    /// The field for the held regions at `values` and the cells' `sources` (per unit length;
    /// empty for none).
    pub(crate) fn solve(&self, values: &[f64], sources: &[f64]) -> Result<Field> {
        if values.len() != self.holds {
            return Err(invalid(format!(
                "{} held regions, {} values",
                self.holds,
                values.len()
            )));
        }
        let value = |t: Target| -> f64 {
            match t {
                Target::Hold(k) => values[k],
                Target::Side(_, v) => v,
            }
        };
        let mut b = vec![c64::new(0.0, 0.0); self.unknowns];
        let mut total_source = 0.0;
        for (c, place) in self.cell.iter().enumerate() {
            if let Place::Free(u) = *place
                && let Some(&s) = sources.get(c)
            {
                b[u].re += s;
                total_source += s;
            }
        }
        for &(u, q, _) in &self.inflows {
            b[u].re += q;
        }
        for &(u, g, t) in &self.links {
            b[u].re += g * value(t);
        }
        let x = match &self.factors {
            Some(f) => f.solve(&b),
            None => Vec::new(),
        };
        let mut cells = vec![0.0; self.cell.len()];
        for (c, place) in self.cell.iter().enumerate() {
            cells[c] = match *place {
                Place::Free(u) => x[u].re,
                Place::Held(k) => values[k],
            };
        }
        let free: Vec<f64> = x.iter().map(|v| v.re).collect();
        let mut held = vec![0.0; self.holds];
        let mut outflow = [0.0; 4];
        for &(u, g, t) in &self.links {
            match t {
                Target::Hold(k) => held[k] += g * (values[k] - free[u]),
                Target::Side(s, v) => outflow[s] += g * (free[u] - v),
            }
        }
        for &(_, q, s) in &self.inflows {
            outflow[s] -= q;
        }
        Ok(Field {
            grid: self.grid.clone(),
            values: cells,
            levels: values.to_vec(),
            held,
            outflow,
            source: total_source,
        })
    }

    /// The power each cell dissipates in `field` (per unit length): each face between free
    /// cells g Δu², half to each side; a face to a held region or a fixed side, all to its
    /// free cell. For current continuity, the Joule heat.
    pub(crate) fn dissipation(&self, field: &Field) -> Vec<f64> {
        let mut out = vec![0.0; self.cell.len()];
        let mut of = vec![0usize; self.unknowns];
        for (c, place) in self.cell.iter().enumerate() {
            if let Place::Free(u) = *place {
                of[u] = c;
            }
        }
        for &(a, b, g) in &self.faces {
            let (ca, cb) = (of[a], of[b]);
            let d = field.values[ca] - field.values[cb];
            let p = g * d * d;
            out[ca] += 0.5 * p;
            out[cb] += 0.5 * p;
        }
        for &(u, g, t) in &self.links {
            let c = of[u];
            let other = match t {
                Target::Hold(k) => field.levels[k],
                Target::Side(_, v) => v,
            };
            let d = field.values[c] - other;
            out[c] += g * d * d;
        }
        out
    }

    /// Each cell's part of the discrete energy Σ g Δu² of `field`: a face between free cells
    /// split between its two half cells in proportion to their resistances d/c, a face to a
    /// held region or a fixed side all in its free cell. A cell's part is c ∂(Σ g Δu²)/∂c at
    /// fixed u, so a medium's parts summed are its coefficient's share of the energy: for
    /// electrostatics, ε_i ∂C/∂ε_i / C, the filling factor a loss tangent weighs.
    pub(crate) fn energies(&self, field: &Field) -> Vec<f64> {
        let mut out = vec![0.0; self.cell.len()];
        let of = self.free_cells();
        for (&(a, b, g), &s) in self.faces.iter().zip(&self.splits) {
            let (ca, cb) = (of[a], of[b]);
            let d = field.values[ca] - field.values[cb];
            let p = g * d * d;
            out[ca] += s * p;
            out[cb] += (1.0 - s) * p;
        }
        for &(u, g, t) in &self.links {
            let c = of[u];
            let other = match t {
                Target::Hold(k) => field.levels[k],
                Target::Side(_, v) => v,
            };
            let d = field.values[c] - other;
            out[c] += g * d * d;
        }
        out
    }

    /// What each held region takes through each face of a free cell it meets in `field`: the
    /// region, the point where the face's line meets it (µm: the face's centre, or a shape's
    /// boundary between the centres), the face's length (µm) and the flux (C/m, W/m or A/m)
    /// into the window from the region through it.
    pub(crate) fn surface(&self, field: &Field) -> Vec<Patch> {
        let nx = self.grid.x.cells();
        let of = self.free_cells();
        let mut out = Vec::new();
        for (&(u, g, t), &f) in self.links.iter().zip(&self.link_faces) {
            let Target::Hold(k) = t else { continue };
            let c = of[u];
            let (i, j) = (c % nx, c / nx);
            let (xc, yc) = (self.grid.x.centre(i), self.grid.y.centre(j));
            let (hx, hy) = (self.grid.x.width(i), self.grid.y.width(j));
            // the distance from the centre to the region, from the conductance L c / d
            let (length, own) = if f < 2 {
                (hy, self.coefficients[c][0])
            } else {
                (hx, self.coefficients[c][1])
            };
            let d = length * own / g;
            let at = match f {
                0 => [xc - d, yc],
                1 => [xc + d, yc],
                2 => [xc, yc - d],
                _ => [xc, yc + d],
            };
            out.push(Patch {
                hold: k,
                at,
                length,
                flux: g * (field.levels[k] - field.values[c]),
            });
        }
        out
    }
}

/// A piece of a held region's surface ([`Discrete::surface`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Patch {
    /// The region.
    pub hold: usize,
    /// Where the face's line meets it, µm.
    pub at: [f64; 2],
    /// The face's length, µm.
    pub length: f64,
    /// The flux into the window through it.
    pub flux: f64,
}

/// A solution on a cross-section: the value at each cell's centre (held cells at their
/// region's value), what flows out of the held regions and through the window's sides, all per
/// unit length along the guide.
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    grid: Grid,
    values: Vec<f64>,
    levels: Vec<f64>,
    held: Vec<f64>,
    outflow: [f64; 4],
    source: f64,
}

impl Field {
    /// The grid.
    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    /// (`self` − `other`) / `by`, every part of it: a rise above a background per unit of
    /// what drives it (the problem being linear). Both on the same grid, which the caller
    /// ensures.
    pub(crate) fn above(&self, other: &Field, by: f64) -> Field {
        let d = |a: &[f64], b: &[f64]| -> Vec<f64> {
            a.iter().zip(b).map(|(a, b)| (a - b) / by).collect()
        };
        let mut outflow = [0.0; 4];
        for (k, o) in outflow.iter_mut().enumerate() {
            *o = (self.outflow[k] - other.outflow[k]) / by;
        }
        Field {
            grid: self.grid.clone(),
            values: d(&self.values, &other.values),
            levels: d(&self.levels, &other.levels),
            held: d(&self.held, &other.held),
            outflow,
            source: (self.source - other.source) / by,
        }
    }

    /// The values at the cells' centres, row by row from the bottom (cell (i, j) at
    /// j · nx + i).
    pub fn values(&self) -> &[f64] {
        &self.values
    }

    /// Cell (i, j)'s value.
    pub fn value(&self, i: usize, j: usize) -> f64 {
        self.values[j * self.grid.x.cells() + i]
    }

    /// The value at (x, y), µm: bilinear between the four nearest cells' centres, the nearest
    /// edge cells' beyond the outermost centres.
    pub fn at(&self, x: f64, y: f64) -> f64 {
        let (gx, gy) = (&self.grid.x, &self.grid.y);
        let locate = |a: &Axis, v: f64| -> (usize, usize, f64) {
            let n = a.cells();
            if n == 1 || v <= a.centre(0) {
                return (0, 0, 0.0);
            }
            if v >= a.centre(n - 1) {
                return (n - 1, n - 1, 0.0);
            }
            let mut i = a.cell_of(v);
            if v < a.centre(i) {
                i -= 1;
            }
            let t = (v - a.centre(i)) / (a.centre(i + 1) - a.centre(i));
            (i, i + 1, t)
        };
        let (i0, i1, s) = locate(gx, x);
        let (j0, j1, t) = locate(gy, y);
        let v = |i, j| self.value(i, j);
        (1.0 - t) * ((1.0 - s) * v(i0, j0) + s * v(i1, j0))
            + t * ((1.0 - s) * v(i0, j1) + s * v(i1, j1))
    }

    /// The area-weighted mean over `shape`'s overlap with the cells.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if `shape` covers no cell.
    pub fn mean(&self, shape: &Shape) -> Result<f64> {
        let parts = self.grid.overlap(shape);
        let area: f64 = parts.iter().map(|p| p.1).sum();
        if area <= 0.0 {
            return Err(invalid("the shape covers no cell of the grid"));
        }
        Ok(parts.iter().map(|&(c, a)| self.values[c] * a).sum::<f64>() / area)
    }

    /// The largest value and the centre (µm) of its cell.
    pub fn max(&self) -> (f64, [f64; 2]) {
        let nx = self.grid.x.cells();
        let (mut best, mut at) = (f64::NEG_INFINITY, 0);
        for (c, &v) in self.values.iter().enumerate() {
            if v > best {
                best = v;
                at = c;
            }
        }
        (
            best,
            [self.grid.x.centre(at % nx), self.grid.y.centre(at / nx)],
        )
    }

    /// The gradient at cell (i, j)'s centre, per metre: central differences between its
    /// neighbours' centres (one-sided at the window's edges).
    pub fn gradient(&self, i: usize, j: usize) -> [f64; 2] {
        let (nx, ny) = (self.grid.x.cells(), self.grid.y.cells());
        let along = |a: &Axis, n: usize, k: usize, v: &dyn Fn(usize) -> f64| -> f64 {
            if n == 1 {
                return 0.0;
            }
            let (lo, hi) = (k.saturating_sub(1), (k + 1).min(n - 1));
            (v(hi) - v(lo)) / (a.centre(hi) - a.centre(lo)) * 1e6
        };
        [
            along(&self.grid.x, nx, i, &|k| self.value(k, j)),
            along(&self.grid.y, ny, j, &|k| self.value(i, k)),
        ]
    }

    /// What flows out of each held region into the window, per unit length: a conductor's
    /// charge (C/m), the heat a sink gives (W/m; negative when it takes heat), a contact's
    /// current (A/m), in the order the problem held them.
    pub fn held(&self) -> &[f64] {
        &self.held
    }

    /// What flows out through `side`, per unit length.
    pub fn outflow(&self, side: Side) -> f64 {
        self.outflow[side.index()]
    }

    /// The sources' total, per unit length.
    pub fn source(&self) -> f64 {
        self.source
    }

    /// How far the balance is from closing: sources plus what the held regions give, less what
    /// leaves through the sides, over the largest of them (0 for no flow at all).
    pub fn imbalance(&self) -> f64 {
        let given: f64 = self.held.iter().sum();
        let out: f64 = self.outflow.iter().sum();
        let scale = self
            .held
            .iter()
            .chain(&self.outflow)
            .chain(std::iter::once(&self.source))
            .fold(0.0f64, |m, v| m.max(v.abs()));
        if scale == 0.0 {
            0.0
        } else {
            (self.source + given - out).abs() / scale
        }
    }

    /// A picture of the field over the window `x` × `y` (µm), `columns` × `rows` pixels, each
    /// the value of the cell its centre falls in (in the field's own units, not scaled).
    pub fn raster(
        &self,
        x: [f64; 2],
        y: [f64; 2],
        columns: usize,
        rows: usize,
    ) -> crate::raster::Raster {
        let (nx, ny) = (columns.max(1), rows.max(1));
        let mut values = Vec::with_capacity(nx * ny);
        for r in 0..ny {
            let yv = y[0] + (r as f64 + 0.5) * (y[1] - y[0]) / ny as f64;
            let j = self.grid.y.cell_of(yv);
            for c in 0..nx {
                let xv = x[0] + (c as f64 + 0.5) * (x[1] - x[0]) / nx as f64;
                values.push(self.value(self.grid.x.cell_of(xv), j) as f32);
            }
        }
        crate::raster::Raster {
            nx,
            ny,
            x0: x[0],
            x1: x[1],
            y0: y[0],
            y1: y[1],
            values,
        }
    }
}

/// Each cell's share of `total` (per unit length) spread over `shape` by area: what a heater or
/// a charge gives each cell. The shares add up to `total` to round-off.
pub(crate) fn spread(grid: &Grid, shape: &Shape, total: f64, into: &mut [f64]) -> Result<()> {
    let parts = grid.overlap(shape);
    let area: f64 = parts.iter().map(|p| p.1).sum();
    if area <= 0.0 {
        return Err(invalid("a source's shape covers no cell of the grid"));
    }
    for (c, a) in parts {
        into[c] += total * a / area;
    }
    Ok(())
}

/// K(k), the complete elliptic integral of the first kind of modulus `k` (0 ≤ k < 1), by the
/// arithmetic-geometric mean: K = π / (2 AGM(1, √(1 − k²))).
pub(crate) fn elliptic_k(k: f64) -> f64 {
    let (mut a, mut b) = (1.0f64, (1.0 - k * k).sqrt());
    for _ in 0..64 {
        let (an, bn) = (0.5 * (a + b), (a * b).sqrt());
        if (an - bn).abs() <= f64::EPSILON * an {
            a = an;
            break;
        }
        a = an;
        b = bn;
    }
    std::f64::consts::PI / (2.0 * a)
}
