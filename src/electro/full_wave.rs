//! A travelling-wave electrode's full-wave RF mode: the line's cross-section solved as a
//! waveguide at the RF frequency, the metal a lossy medium, for the range where quasi-TEM
//! ([`super::line::QuasiTem`]) no longer holds (docs/methods/travelling-wave.md).
//!
//! **The formulation.** The cross-section of an [`Electrostatic`] problem (its section, grid,
//! media and conductors), on its own graded grid, as a Yee lattice (K. S. Yee, IEEE Trans.
//! Antennas Propag. 14, 302 (1966), doi:10.1109/TAP.1966.1138693): H_x on the grid's vertical
//! edges, H_y on its horizontal ones, H_z and ∇·H at the cells' centres, E_z and (∇ × H)_z at
//! the nodes. With fields as e^(i(βz − ωt)), ∇·H = 0 gives H_z = i∇_t·H_t/β, and
//! ∇ × (ε⁻¹ ∇ × H) = k₀²H leaves an eigenproblem for the transverse H, linear in β²:
//!
//! β² H_x = k₀² ε_y H_x + ∂_x(∇_t·H_t) − ε_y ∂_y(ε_z⁻¹ (∂_x H_y − ∂_y H_x)),
//! β² H_y = k₀² ε_x H_y + ∂_y(∇_t·H_t) + ε_x ∂_x(ε_z⁻¹ (∂_x H_y − ∂_y H_x)),
//!
//! each difference the lattice's own. This is the transverse-H formulation of a vector mode
//! solver; Fallahkhair et al.'s nodal scheme ([`crate::mode::vector`]) solves the same equations
//! at the optical wavelengths, where β² is near k₀²ε_max. At RF it can't: a line's β² is
//! 1e-7 µm⁻² against entries of 1/h², and the quasi-TEM mode lives on the cancellation of the
//! transverse operator's static part, which a nodal scheme leaves to its truncation error
//! (larger than β² itself on a cross-section of microns). On the lattice the discrete
//! divergence of a discrete curl vanishes and mixed differences commute, so a TEM field
//! (H_t = ẑ × ∇φ with φ discretely harmonic) is an exact eigenvector, β² = k₀²ε exactly, and
//! what moves β² from k₀²ε is the line's physics: the dielectrics' inhomogeneity and the
//! metal's internal impedance.
//!
//! **The media.** Each dielectric is ε_r(1 + i tan δ) (photonoxide's e^(−iωt)), and the
//! conductors are a medium of permittivity 1 + iσ/(ωε₀): the current flows inside the metal and
//! its skin depth δ = √(2/(ωμ₀σ)) is resolved by the grid, the static solver's graded grid fine
//! at the metal's faces and corners. This, not a surface impedance, because the electrodes are
//! only a few skin depths thick (0.8 µm of gold is 3.2 δ at 100 GHz and 5.5 δ at 300 GHz) and
//! their corners are sharper than δ: a Leontovich impedance needs both the thickness and the
//! radius of curvature well above δ (Wheeler's rule asks twice), and it misses the current that
//! reaches through the metal from one face to the other, where W. Heinrich, IEEE Trans. Microw.
//! Theory Tech. 41, 45 (1993), doi:10.1109/22.210228, finds a coplanar line's resistance and
//! inductance leave their skin-effect forms. The volume current holds from a few GHz to the THz.
//! ε at an edge is the mean of the two cells beside it (the E it multiplies is tangential to the
//! interface between them), at a node the mean of four.
//!
//! **The window's sides** are the static problem's: an insulated side (a mirror, D·n = 0) is a
//! magnetic wall (the tangential H odd across it), a side at a fixed potential an electric wall
//! (the normal H odd). An [`Open`] side continues the window beyond it, its edge's media
//! running on, into a perfectly matched layer of complex-stretched coordinates (W. C. Chew, W.
//! H. Weedon, Microw. Opt. Technol. Lett. 7, 599 (1994), doi:10.1002/mop.4650071304), ending on
//! an electric wall, so a wave the line radiates (into a substrate whose plane waves are slower
//! than the line: the shock wave whose loss grows as f³ at first, M. Y. Frankel et al., IEEE
//! Trans. Microw. Theory Tech. 39, 910 (1991), doi:10.1109/22.81658) leaves the window and
//! shows as the mode's loss.
//!
//! **The mode** is the one nearest the quasi-TEM index that carries the most current on the
//! signal for its field (the substrate's own modes and the box's carry little). From it:
//!
//! - γ = α + jβ from its complex propagation constant (photonoxide's e^(i(βz − ωt)) turned into
//!   the papers' e^(jωt − γz)), n_m = βc/ω;
//! - the signal's current I = ∮ H·dl, the sum of (∇ × H)_z over the nodes of the signal and the
//!   ring of nodes around it (on the lattice, exactly H's circulation half a cell outside);
//! - the complex power P = ½ ∫ (E × H*)·ẑ dA outside the open sides' layers, E from Ampère's
//!   law on the lattice;
//! - Z₀ = 2P/|I|², the power–current impedance (conjugated into e^(jωt)), which for a TEM
//!   line is V/I, and which meets quasi-TEM's √(Z/Y) where both hold
//!   (`line/full-wave-quasi-tem-impedance`, 0.1% at 50 GHz);
//! - the series impedance γZ₀ = R + jωL and the shunt admittance γ/Z₀ = G + jωC;
//! - the loss split by where the power goes, ½ ∫ ωε₀ ε'' |E|² dA in the metal and in the
//!   dielectrics each over 2 Re P, and what the eigenvalue's α holds beyond them: the power
//!   radiated into the open sides.

use num_complex::Complex64 as c64;

use super::line::{DB_PER_NEPER, mu_0};
use super::{EPSILON_0, Electrostatic};
use crate::statics::{Boundary, Hold, Side};
use crate::thermal::side_index;
use crate::units::SPEED_OF_LIGHT;
use crate::{Error, Result};

fn invalid(reason: impl Into<String>) -> Error {
    Error::invalid("full-wave line", reason)
}

/// The eigensolver's tolerance on ‖Ah − β²h‖ relative to |β²|.
const TOLERANCE: f64 = 1e-10;

/// An open side of the window: the window continued `thickness_um` beyond it in `cells` equal
/// cells, the media at its edge running on, all of it a perfectly matched layer.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Open {
    /// The side.
    pub side: Side,
    /// The layer's thickness, µm.
    pub thickness_um: f64,
    /// The layer's cells.
    pub cells: usize,
}

impl Open {
    /// `side` continued by a layer `thickness_um` thick in 10 cells.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless the thickness is finite and positive.
    pub fn new(side: Side, thickness_um: f64) -> Result<Open> {
        if !(thickness_um.is_finite() && thickness_um > 0.0) {
            return Err(invalid(format!(
                "an open side's layer must be thicker than 0, got {thickness_um} µm"
            )));
        }
        Ok(Open {
            side,
            thickness_um,
            cells: 10,
        })
    }

    /// The same layer in `cells` cells (at least 2).
    pub fn with_cells(mut self, cells: usize) -> Open {
        self.cells = cells.max(2);
        self
    }
}

/// How to solve a line's full-wave mode: its signal, its metal, the dielectrics' loss
/// tangents, the permittivity along the line where it differs from the one up, and the open
/// sides. See the module's docs.
#[derive(Clone, Debug, PartialEq)]
pub struct FullWave {
    signal: usize,
    copies: f64,
    conductivity: f64,
    tangents: Vec<(String, f64)>,
    along: Vec<(String, f64)>,
    open: Vec<Open>,
    strength: f64,
    count: usize,
}

/// A side of the lattice: an electric or a magnetic wall.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Wall {
    Electric,
    Magnetic,
}

impl Wall {
    /// The sign across the wall of a field component tangential to it (H_t, H_z, ∇·H):
    /// even under an electric wall, odd under a magnetic one.
    fn tangential(self) -> f64 {
        match self {
            Wall::Electric => 1.0,
            Wall::Magnetic => -1.0,
        }
    }
}

/// The cross-section on its Yee lattice: the real and stretched nodes and centres, each cell's
/// permittivity (across, up, along), its metal and whether it is inside the window (not in an
/// open side's layer), and the four walls (left, right, bottom, top).
struct Lattice {
    nx: usize,
    ny: usize,
    x: Vec<f64>,
    y: Vec<f64>,
    xs: Vec<c64>,
    ys: Vec<c64>,
    xc: Vec<c64>,
    yc: Vec<c64>,
    eps: Vec<[c64; 3]>,
    metal: Vec<Option<(usize, f64)>>,
    inside: Vec<bool>,
    walls: [Wall; 4],
    hx: Vec<Option<usize>>,
    hy: Vec<Option<usize>>,
    unknowns: usize,
}

/// A wave's coordinate stretched into the complex plane inside the layers at either end
/// (Chew and Weedon): x + iα u³/(3d²) at depth u into a layer d thick, its sign outward.
fn stretched(v: f64, start: f64, end: f64, low: f64, high: f64, strength: f64) -> c64 {
    let mut im = 0.0;
    if low > 0.0 && v < start + low {
        let u = start + low - v;
        im -= strength * u.powi(3) / (3.0 * low * low);
    }
    if high > 0.0 && v > end - high {
        let u = v - (end - high);
        im += strength * u.powi(3) / (3.0 * high * high);
    }
    c64::new(v, im)
}

impl Lattice {
    fn cell(&self, i: usize, j: usize) -> usize {
        j * self.nx + i
    }

    /// The primal spacing of cell i along x (stretched).
    fn dx(&self, i: usize) -> c64 {
        self.xs[i + 1] - self.xs[i]
    }

    fn dy(&self, j: usize) -> c64 {
        self.ys[j + 1] - self.ys[j]
    }

    /// The dual spacing at node i along x: centre to centre, a wall's ghost centre its mirror.
    fn dxd(&self, i: usize) -> c64 {
        let lo = if i == 0 {
            2.0 * self.xs[0] - self.xc[0]
        } else {
            self.xc[i - 1]
        };
        let hi = if i == self.nx {
            2.0 * self.xs[self.nx] - self.xc[self.nx - 1]
        } else {
            self.xc[i]
        };
        hi - lo
    }

    fn dyd(&self, j: usize) -> c64 {
        let lo = if j == 0 {
            2.0 * self.ys[0] - self.yc[0]
        } else {
            self.yc[j - 1]
        };
        let hi = if j == self.ny {
            2.0 * self.ys[self.ny] - self.yc[self.ny - 1]
        } else {
            self.yc[j]
        };
        hi - lo
    }

    /// The cells around a location, with each one's share of the location's dual area (real
    /// lengths): (cell, area µm²). `ix`/`iy` are the location's index along each axis and
    /// whether it sits on a node line (then the cells on both sides share it) or mid-cell.
    fn around(&self, ix: (usize, bool), iy: (usize, bool)) -> Vec<(usize, f64)> {
        let span = |n: usize, nodes: &[f64], (k, on_line): (usize, bool)| -> Vec<(usize, f64)> {
            if on_line {
                let mut v = Vec::with_capacity(2);
                if k > 0 {
                    v.push((k - 1, 0.5 * (nodes[k] - nodes[k - 1])));
                }
                if k < n {
                    v.push((k, 0.5 * (nodes[k + 1] - nodes[k])));
                }
                v
            } else {
                vec![(k, nodes[k + 1] - nodes[k])]
            }
        };
        let (sx, sy) = (span(self.nx, &self.x, ix), span(self.ny, &self.y, iy));
        let mut out = Vec::with_capacity(4);
        for &(i, wx) in &sx {
            for &(j, wy) in &sy {
                out.push((self.cell(i, j), wx * wy));
            }
        }
        out
    }

    /// The mean of component `a` of the cells `cells` share, by area.
    fn mean(&self, cells: &[(usize, f64)], a: usize) -> c64 {
        let total: f64 = cells.iter().map(|c| c.1).sum();
        cells.iter().map(|&(c, w)| self.eps[c][a] * w).sum::<c64>() / total
    }

    /// ε_x where H_y (and E_x) is: (i + ½, j).
    fn eps_x(&self, i: usize, j: usize) -> c64 {
        self.mean(&self.around((i, false), (j, true)), 0)
    }

    /// ε_y where H_x (and E_y) is: (i, j + ½).
    fn eps_y(&self, i: usize, j: usize) -> c64 {
        self.mean(&self.around((i, true), (j, false)), 1)
    }

    /// ε_z at node (i, j).
    fn eps_z(&self, i: usize, j: usize) -> c64 {
        self.mean(&self.around((i, true), (j, true)), 2)
    }

    /// The unknown of H_x at (i, j + ½), signed for a ghost beyond a bottom or top wall.
    fn hx_at(&self, i: usize, j: isize) -> Option<(usize, f64)> {
        let (j, sign) = self.fold(j, self.ny, 2, 3);
        self.hx[i * self.ny + j].map(|k| (k, sign))
    }

    /// The unknown of H_y at (i + ½, j), signed for a ghost beyond a left or right wall.
    fn hy_at(&self, i: isize, j: usize) -> Option<(usize, f64)> {
        let (i, sign) = self.fold(i, self.nx, 0, 1);
        self.hy[i * (self.ny + 1) + j].map(|k| (k, sign))
    }

    /// A cell index one beyond either end folded back across that end's wall, with the sign
    /// of a component tangential to it.
    fn fold(&self, k: isize, n: usize, low: usize, high: usize) -> (usize, f64) {
        if k < 0 {
            (0, self.walls[low].tangential())
        } else if k as usize >= n {
            (n - 1, self.walls[high].tangential())
        } else {
            (k as usize, 1.0)
        }
    }

    /// ∇_t·H_t at cell (ci, cj) (one beyond either end: the mirror cell's, signed), as terms.
    fn div(&self, ci: isize, cj: isize) -> Vec<(usize, c64)> {
        let (i, si) = self.fold(ci, self.nx, 0, 1);
        let (j, sj) = self.fold(cj, self.ny, 2, 3);
        let s = si * sj;
        let mut out = Vec::with_capacity(4);
        let (dx, dy) = (self.dx(i), self.dy(j));
        for (k, c) in [(i + 1, 1.0), (i, -1.0)] {
            if let Some(u) = self.hx[k * self.ny + j] {
                out.push((u, c64::new(s * c, 0.0) / dx));
            }
        }
        for (k, c) in [(j + 1, 1.0), (j, -1.0)] {
            if let Some(u) = self.hy[i * (self.ny + 1) + k] {
                out.push((u, c64::new(s * c, 0.0) / dy));
            }
        }
        out
    }

    /// Whether node (i, j) lies on an electric wall, where E_z, and so (∇ × H)_z, is zero.
    fn on_electric(&self, i: usize, j: usize) -> bool {
        (i == 0 && self.walls[0] == Wall::Electric)
            || (i == self.nx && self.walls[1] == Wall::Electric)
            || (j == 0 && self.walls[2] == Wall::Electric)
            || (j == self.ny && self.walls[3] == Wall::Electric)
    }

    /// (∇ × H)_z = ∂_x H_y − ∂_y H_x at node (i, j), as terms.
    fn curl(&self, i: usize, j: usize) -> Vec<(usize, c64)> {
        if self.on_electric(i, j) {
            return Vec::new();
        }
        let mut out = Vec::with_capacity(4);
        let (dxd, dyd) = (self.dxd(i), self.dyd(j));
        for (k, c) in [(i as isize, 1.0), (i as isize - 1, -1.0)] {
            if let Some((u, s)) = self.hy_at(k, j) {
                out.push((u, c64::new(s * c, 0.0) / dxd));
            }
        }
        for (k, c) in [(j as isize, -1.0), (j as isize - 1, 1.0)] {
            if let Some((u, s)) = self.hx_at(i, k) {
                out.push((u, c64::new(s * c, 0.0) / dyd));
            }
        }
        out
    }

    /// The matrix of the eigenproblem for β² at k₀ = `k` (1/µm), as (row, column, value).
    fn assemble(&self, k: f64) -> Vec<(usize, usize, c64)> {
        let k2 = k * k;
        let mut entries = Vec::with_capacity(self.unknowns * 20);
        let mut push = |row: usize, terms: &[(usize, c64)], scale: c64| {
            for &(u, v) in terms {
                entries.push((row, u, v * scale));
            }
        };
        let (nx, ny) = (self.nx, self.ny);
        for i in 0..=nx {
            for j in 0..ny {
                let Some(row) = self.hx[i * ny + j] else {
                    continue;
                };
                let ey = self.eps_y(i, j);
                push(row, &[(row, ey * k2)], c64::new(1.0, 0.0));
                let dxd = self.dxd(i);
                push(row, &self.div(i as isize, j as isize), 1.0 / dxd);
                push(row, &self.div(i as isize - 1, j as isize), -1.0 / dxd);
                let dy = self.dy(j);
                push(row, &self.curl(i, j + 1), -ey / (self.eps_z(i, j + 1) * dy));
                push(row, &self.curl(i, j), ey / (self.eps_z(i, j) * dy));
            }
        }
        for i in 0..nx {
            for j in 0..=ny {
                let Some(row) = self.hy[i * (ny + 1) + j] else {
                    continue;
                };
                let ex = self.eps_x(i, j);
                push(row, &[(row, ex * k2)], c64::new(1.0, 0.0));
                let dyd = self.dyd(j);
                push(row, &self.div(i as isize, j as isize), 1.0 / dyd);
                push(row, &self.div(i as isize, j as isize - 1), -1.0 / dyd);
                let dx = self.dx(i);
                push(row, &self.curl(i + 1, j), ex / (self.eps_z(i + 1, j) * dx));
                push(row, &self.curl(i, j), -ex / (self.eps_z(i, j) * dx));
            }
        }
        entries
    }
}

/// Applies `terms` to the field `h`.
fn apply(terms: &[(usize, c64)], h: &[c64]) -> c64 {
    terms.iter().map(|&(u, v)| v * h[u]).sum()
}

impl FullWave {
    /// The line whose signal is conductor `signal` (counted from 0, as
    /// [`Electrostatic::conductors`] has them; the others are its grounds), its metal of
    /// `conductivity` (S/m), every dielectric lossless, the window closed.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless the conductivity is finite and positive.
    pub fn new(signal: usize, conductivity: f64) -> Result<FullWave> {
        if !(conductivity.is_finite() && conductivity > 0.0) {
            return Err(invalid(format!(
                "the metal needs a positive conductivity, got {conductivity} S/m"
            )));
        }
        Ok(FullWave {
            signal,
            copies: 1.0,
            conductivity,
            tangents: Vec::new(),
            along: Vec::new(),
            open: Vec::new(),
            strength: 4.0,
            count: 3,
        })
    }

    /// For a window that holds half the line, its left side the line's symmetry plane through
    /// the signal (insulated in the static problem: a magnetic wall here, the even mode of a
    /// ground–signal–ground line). The numbers are the whole line's.
    pub fn mirrored(mut self) -> FullWave {
        self.copies = 2.0;
        self
    }

    /// Each medium's loss tangent, by name.
    pub fn with_loss_tangents(mut self, tangents: &[(&str, f64)]) -> FullWave {
        self.tangents = tangents
            .iter()
            .map(|(m, t)| ((*m).to_owned(), *t))
            .collect();
        self
    }

    /// The relative permittivity along the line (ε_zz) of `medium`, where it differs from the
    /// one up (ε_yy, which it is by default: x-cut lithium niobate with the line along its y,
    /// whose ordinary ε₁₁ is both).
    pub fn with_along(mut self, medium: &str, relative: f64) -> FullWave {
        self.along.retain(|(m, _)| m != medium);
        self.along.push((medium.to_owned(), relative));
        self
    }

    /// The window opened at `open`'s side.
    pub fn with_open(mut self, open: Open) -> FullWave {
        self.open.retain(|o| o.side != open.side);
        self.open.push(open);
        self
    }

    /// The perfectly matched layers' strength α (s = 1 + iα at the window's edge; 4 by
    /// default).
    pub fn with_strength(mut self, strength: f64) -> FullWave {
        self.strength = strength;
        self
    }

    /// How many modes nearest the quasi-TEM index to look among (3 by default).
    pub fn with_count(mut self, count: usize) -> FullWave {
        self.count = count.max(1);
        self
    }

    /// The lattice of `problem` at `frequency_hz`.
    fn lattice(&self, problem: &Electrostatic, frequency_hz: f64) -> Result<Lattice> {
        let w = 2.0 * std::f64::consts::PI * frequency_hz;
        let grid = &problem.grid;
        let (gx, gy) = (grid.x().cells(), grid.y().cells());
        let media = problem.section.media();
        if let Some((m, _)) = self
            .tangents
            .iter()
            .chain(&self.along)
            .find(|(m, _)| !media.contains(m))
        {
            return Err(invalid(format!(
                "{m} isn't a medium of the section (it has {})",
                media.join(", ")
            )));
        }
        // each medium's complex permittivity, ε_r(1 + i tan δ): across, up, along
        let mut table = Vec::with_capacity(media.len());
        for m in media {
            let t = super::table(&problem.permittivity, m)
                .ok_or_else(|| invalid(format!("the medium {m} has no permittivity")))?;
            let tan = self
                .tangents
                .iter()
                .find(|(n, _)| n == m)
                .map_or(0.0, |(_, t)| *t);
            let zz = self
                .along
                .iter()
                .find(|(n, _)| n == m)
                .map_or(t.yy, |(_, v)| *v);
            let lossy = |e: f64| c64::new(e, e * tan);
            table.push([lossy(t.xx), lossy(t.yy), lossy(zz)]);
        }
        if self.signal >= problem.conductors.len() || problem.conductors.len() < 2 {
            return Err(invalid(format!(
                "the signal is conductor {} of {}: a line needs a signal and a ground",
                self.signal,
                problem.conductors.len()
            )));
        }
        let metal = c64::new(1.0, self.conductivity / (w * EPSILON_0));
        // each conductor's share of each cell
        let mut owner: Vec<Option<(usize, f64)>> = vec![None; gx * gy];
        for (k, c) in problem.conductors.iter().enumerate() {
            let Hold::Shape(shape) = &c.hold else {
                return Err(invalid(
                    "a conductor of zero thickness has no metal for the current to flow in: draw \
                     it as a shape",
                ));
            };
            for (cell, area) in grid.overlap(shape) {
                let (i, j) = (cell % gx, cell / gx);
                let f = (area / (grid.x().width(i) * grid.y().width(j))).min(1.0);
                if owner[cell].is_none_or(|(_, g)| f > g) {
                    owner[cell] = Some((k, f));
                }
            }
        }
        let shares = problem.section.shares_on(grid);
        let mut eps = Vec::with_capacity(gx * gy);
        for (s, o) in shares.iter().zip(&owner) {
            let f = o.map_or(0.0, |o| o.1);
            let mut parts: Vec<([c64; 3], f64)> = s
                .parts
                .iter()
                .map(|&(m, g)| (table[m], g * (1.0 - f)))
                .filter(|p| p.1 > 0.0)
                .collect();
            let mut normal = s.normal;
            if f > 0.0 {
                parts.push(([metal; 3], f));
                normal = None;
            }
            eps.push(average(&parts, normal));
        }
        // the open sides: the edge's cells run on into the layers
        let open = |side: Side| self.open.iter().find(|o| o.side == side);
        let extend = |nodes: &[f64], low: Option<&Open>, high: Option<&Open>| -> Vec<f64> {
            let mut v = Vec::new();
            if let Some(o) = low {
                let h = o.thickness_um / o.cells as f64;
                v.extend((1..=o.cells).rev().map(|c| nodes[0] - h * c as f64));
            }
            v.extend_from_slice(nodes);
            if let Some(o) = high {
                let h = o.thickness_um / o.cells as f64;
                let end = nodes[nodes.len() - 1];
                v.extend((1..=o.cells).map(|c| end + h * c as f64));
            }
            v
        };
        let x = extend(grid.x().nodes(), open(Side::Left), open(Side::Right));
        let y = extend(grid.y().nodes(), open(Side::Bottom), open(Side::Top));
        let west = open(Side::Left).map_or(0, |o| o.cells);
        let south = open(Side::Bottom).map_or(0, |o| o.cells);
        let (nx, ny) = (x.len() - 1, y.len() - 1);
        let thickness = |side: Side| open(side).map_or(0.0, |o| o.thickness_um);
        let stretch = |v: &[f64], low: f64, high: f64| -> (Vec<c64>, Vec<c64>) {
            let (a, b) = (v[0], v[v.len() - 1]);
            let s = |p: f64| stretched(p, a, b, low, high, self.strength);
            (
                v.iter().map(|&p| s(p)).collect(),
                v.windows(2).map(|w| s(0.5 * (w[0] + w[1]))).collect(),
            )
        };
        let (xs, xc) = stretch(&x, thickness(Side::Left), thickness(Side::Right));
        let (ys, yc) = stretch(&y, thickness(Side::Bottom), thickness(Side::Top));
        let mut all = Vec::with_capacity(nx * ny);
        let mut metals = Vec::with_capacity(nx * ny);
        let mut inside = Vec::with_capacity(nx * ny);
        for b in 0..ny {
            for a in 0..nx {
                let i = a.saturating_sub(west).min(gx - 1);
                let j = b.saturating_sub(south).min(gy - 1);
                all.push(eps[j * gx + i]);
                let within = a >= west && a < west + gx && b >= south && b < south + gy;
                inside.push(within);
                metals.push(if within { owner[j * gx + i] } else { None });
            }
        }
        let wall = |side: Side| -> Result<Wall> {
            if open(side).is_some() {
                return Ok(Wall::Electric);
            }
            match problem.sides[side_index(side)] {
                Boundary::Flux(0.0) => Ok(Wall::Magnetic),
                Boundary::Fixed(_) => Ok(Wall::Electric),
                other => Err(invalid(format!(
                    "the {} side holds {other:?}: a full-wave line's sides are insulated (a \
                     magnetic wall), at a fixed potential (an electric wall) or open",
                    side.name()
                ))),
            }
        };
        let walls = [
            wall(Side::Left)?,
            wall(Side::Right)?,
            wall(Side::Bottom)?,
            wall(Side::Top)?,
        ];
        if self.copies > 1.0 && (walls[0] != Wall::Magnetic || open(Side::Left).is_some()) {
            return Err(invalid(
                "a mirrored line's left side is its symmetry plane: insulated, not open or held",
            ));
        }
        // the unknowns: H normal to an electric wall is zero on it
        let mut unknowns = 0;
        let mut hx = vec![None; (nx + 1) * ny];
        for i in 0..=nx {
            for j in 0..ny {
                let held = (i == 0 && walls[0] == Wall::Electric)
                    || (i == nx && walls[1] == Wall::Electric);
                if !held {
                    hx[i * ny + j] = Some(unknowns);
                    unknowns += 1;
                }
            }
        }
        let mut hy = vec![None; nx * (ny + 1)];
        for i in 0..nx {
            for j in 0..=ny {
                let held = (j == 0 && walls[2] == Wall::Electric)
                    || (j == ny && walls[3] == Wall::Electric);
                if !held {
                    hy[i * (ny + 1) + j] = Some(unknowns);
                    unknowns += 1;
                }
            }
        }
        Ok(Lattice {
            nx,
            ny,
            x,
            y,
            xs,
            ys,
            xc,
            yc,
            eps: all,
            metal: metals,
            inside,
            walls,
            hx,
            hy,
            unknowns,
        })
    }

    /// The line's full-wave mode at `frequency_hz` in `problem`'s cross-section, the mode
    /// nearest the index `near` (the quasi-TEM line's is a good guess) that carries the most
    /// current on the signal. See the module's docs.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a frequency or guess that isn't finite and positive, a
    /// conductor of zero thickness, a side the solver can't take, or an eigensolve that fails.
    pub fn solve(&self, problem: &Electrostatic, frequency_hz: f64, near: f64) -> Result<RfMode> {
        Ok(self.picked(problem, frequency_hz, near)?.0.mode)
    }

    /// The line's full-wave mode as [`FullWave::solve`] finds it, with its electric field on
    /// `problem`'s own grid (the open sides' layers left out), scaled to 1 V on the signal: the
    /// mode over its voltage V = 2P/I*, P its complex power and I the signal's current, the
    /// voltage whose Z₀ = V/I is [`RfMode::impedance`] (for a TEM line, the signal's potential
    /// against the ground's). See [`RfField`].
    ///
    /// # Errors
    ///
    /// As [`FullWave::solve`], and a mode that carries no power or no current on the signal.
    pub fn field(&self, problem: &Electrostatic, frequency_hz: f64, near: f64) -> Result<RfField> {
        let (m, lattice) = self.picked(problem, frequency_hz, near)?;
        if m.voltage.norm() == 0.0 || !m.voltage.is_finite() {
            return Err(invalid(
                "the mode carries no power or no current on the signal: it has no voltage",
            ));
        }
        let l = &lattice;
        let grid = &problem.grid;
        let (gx, gy) = (grid.x().cells(), grid.y().cells());
        let west = l.x.partition_point(|&v| v < grid.x().nodes()[0]);
        let south = l.y.partition_point(|&v| v < grid.y().nodes()[0]);
        let ny = l.ny;
        let eta = (mu_0() / EPSILON_0).sqrt();
        let scale = eta / m.voltage;
        let mut e = Vec::with_capacity(gx * gy);
        for b in south..south + gy {
            for a in west..west + gx {
                // E_x on the cell's bottom and top edges, E_y on its left and right, E_z at its
                // corners
                let ex = 0.5 * (m.ex[a * (ny + 1) + b] + m.ex[a * (ny + 1) + b + 1]);
                let ey = 0.5 * (m.ey[a * ny + b] + m.ey[(a + 1) * ny + b]);
                let node = |i: usize, j: usize| m.ez[i * (ny + 1) + j];
                let ez = 0.25 * (node(a, b) + node(a + 1, b) + node(a, b + 1) + node(a + 1, b + 1));
                e.push([ex * scale, ey * scale, ez * scale]);
            }
        }
        Ok(RfField {
            mode: m.mode,
            x: grid.x().nodes().to_vec(),
            y: grid.y().nodes().to_vec(),
            e,
        })
    }

    /// The mode [`FullWave::solve`] picks, measured, and its lattice.
    fn picked(
        &self,
        problem: &Electrostatic,
        frequency_hz: f64,
        near: f64,
    ) -> Result<(Measured, Lattice)> {
        if !(frequency_hz.is_finite() && frequency_hz > 0.0 && near.is_finite() && near > 0.0) {
            return Err(invalid(format!(
                "a frequency and an index to look near must be positive, got {frequency_hz} Hz \
                 and {near}"
            )));
        }
        let lattice = self.lattice(problem, frequency_hz)?;
        let k = 2.0 * std::f64::consts::PI * frequency_hz / SPEED_OF_LIGHT * 1e-6;
        let entries = lattice.assemble(k);
        let shift = c64::new(k * k * near * near, 0.0);
        let pairs =
            crate::eigen::nearest_small(lattice.unknowns, &entries, shift, self.count, TOLERANCE)?;
        let mut best: Option<Measured> = None;
        for p in &pairs {
            let rf = self.measure(&lattice, p.value, &p.vector, frequency_hz);
            if best
                .as_ref()
                .is_none_or(|b| rf.mode.current_per_field > b.mode.current_per_field)
            {
                best = Some(rf);
            }
        }
        let best = best.ok_or_else(|| invalid("the eigensolver found no mode"))?;
        Ok((best, lattice))
    }

    /// The numbers of the mode `h` of eigenvalue `beta2` (µm⁻²) on `lattice`, and its electric
    /// field over η (E_x on the horizontal edges, E_y on the vertical ones, E_z at the nodes).
    fn measure(&self, lattice: &Lattice, beta2: c64, h: &[c64], frequency_hz: f64) -> Measured {
        let l = lattice;
        let (nx, ny) = (l.nx, l.ny);
        let w = 2.0 * std::f64::consts::PI * frequency_hz;
        let k = w / SPEED_OF_LIGHT * 1e-6;
        let eta = (mu_0() / EPSILON_0).sqrt();
        let i1 = c64::new(0.0, 1.0);
        let mut beta = beta2.sqrt();
        if beta.re < 0.0 {
            beta = -beta;
        }
        // H_z at the cells' centres, (∇ × H)_z at the nodes
        let hz = |ci: isize, cj: isize| i1 * apply(&l.div(ci, cj), h) / beta;
        let curl: Vec<c64> = (0..=nx)
            .flat_map(|i| (0..=ny).map(move |j| (i, j)))
            .map(|(i, j)| apply(&l.curl(i, j), h))
            .collect();
        let curl_at = |i: usize, j: usize| curl[i * (ny + 1) + j];
        let value = |u: Option<usize>| u.map_or(c64::new(0.0, 0.0), |u| h[u]);
        // the area of a location's cells inside the window
        let inside = |cells: &[(usize, f64)]| -> f64 {
            cells
                .iter()
                .filter(|&&(c, _)| l.inside[c])
                .map(|&(_, a)| a)
                .sum()
        };
        let (mut power, mut current) = (c64::new(0.0, 0.0), c64::new(0.0, 0.0));
        let (mut in_metal, mut in_dielectric, mut field) = (0.0, 0.0, 0.0);
        let zero = c64::new(0.0, 0.0);
        let mut ex_at = vec![zero; nx * (ny + 1)];
        let mut ey_at = vec![zero; (nx + 1) * ny];
        let mut ez_at = vec![zero; (nx + 1) * (ny + 1)];
        // the power E absorbs at a location, ½ωε₀ε''|E|² over its cells' shares (component
        // `a` of each cell's ε), the metal's apart from the dielectrics'
        let mut absorb = |cells: &[(usize, f64)], a: usize, e: c64| {
            let scale = 0.5 * w * EPSILON_0 * eta * eta * e.norm_sqr() * 1e-12;
            for &(c, area) in cells {
                if !l.inside[c] {
                    continue;
                }
                let p = scale * l.eps[c][a].im * area;
                if l.metal[c].is_some() {
                    in_metal += p;
                } else {
                    in_dielectric += p;
                }
            }
        };
        // E_x with H_y at (i + ½, j): D_x = ∂_y H_z − iβ H_y
        for i in 0..nx {
            for j in 0..=ny {
                let cells = l.around((i, false), (j, true));
                let area = inside(&cells);
                if area == 0.0 {
                    continue;
                }
                let hyv = value(l.hy[i * (ny + 1) + j]);
                let dhz = (hz(i as isize, j as isize) - hz(i as isize, j as isize - 1)) / l.dyd(j);
                let eps = l.eps_x(i, j);
                let ex = i1 * (dhz - i1 * beta * hyv) / (k * eps);
                ex_at[i * (ny + 1) + j] = ex;
                power += 0.5 * eta * ex * hyv.conj() * area * 1e-12;
                absorb(&cells, 0, ex);
                field += hyv.norm_sqr() * area;
            }
        }
        // E_y with H_x at (i, j + ½): D_y = iβ H_x − ∂_x H_z
        for i in 0..=nx {
            for j in 0..ny {
                let cells = l.around((i, true), (j, false));
                let area = inside(&cells);
                if area == 0.0 {
                    continue;
                }
                let hxv = value(l.hx[i * ny + j]);
                let dhz = (hz(i as isize, j as isize) - hz(i as isize - 1, j as isize)) / l.dxd(i);
                let eps = l.eps_y(i, j);
                let ey = i1 * (i1 * beta * hxv - dhz) / (k * eps);
                ey_at[i * ny + j] = ey;
                power -= 0.5 * eta * ey * hxv.conj() * area * 1e-12;
                absorb(&cells, 1, ey);
                field += hxv.norm_sqr() * area;
            }
        }
        // E_z at the nodes, and the signal's current through the nodes that touch it
        for i in 0..=nx {
            for j in 0..=ny {
                let cells = l.around((i, true), (j, true));
                let area = inside(&cells);
                if area == 0.0 {
                    continue;
                }
                let c = curl_at(i, j);
                let eps = l.eps_z(i, j);
                let ez = i1 * c / (k * eps);
                ez_at[i * (ny + 1) + j] = ez;
                absorb(&cells, 2, ez);
                if cells
                    .iter()
                    .any(|&(cell, _)| l.metal[cell].is_some_and(|(m, _)| m == self.signal))
                {
                    current += c * area * 1e-6;
                }
            }
        }
        // e^(i(βz − ωt)) → e^(jωt − γz): γ = Im β + j Re β (1/m), Z₀ conjugated
        let gamma = c64::new(beta.im, beta.re) * 1e6;
        let (impedance, per_field, conductor, dielectric) =
            if current.norm() > 0.0 && power.re > 0.0 {
                (
                    (2.0 * power / (self.copies * current.norm_sqr())).conj(),
                    current.norm_sqr() / field,
                    in_metal / (2.0 * power.re),
                    in_dielectric / (2.0 * power.re),
                )
            } else {
                (c64::new(0.0, 0.0), 0.0, 0.0, 0.0)
            };
        Measured {
            mode: RfMode {
                frequency_hz,
                gamma,
                impedance,
                conductor,
                dielectric,
                current_per_field: per_field,
            },
            // the voltage V = 2P/I* (photonoxide's convention; P and I the window's, so the
            // whole line's for a mirrored one too)
            voltage: if current.norm() > 0.0 {
                2.0 * power / current.conj()
            } else {
                zero
            },
            ex: ex_at,
            ey: ey_at,
            ez: ez_at,
        }
    }
}

/// A mode as [`FullWave::measure`] gives it: its numbers, its voltage (V) and its field over η.
struct Measured {
    mode: RfMode,
    voltage: c64,
    ex: Vec<c64>,
    ey: Vec<c64>,
    ez: Vec<c64>,
}

/// A line's full-wave mode with its electric field ([`FullWave::field`]), in photonoxide's
/// e^(i(βz − ωt)): E at the centre of each cell of the static problem's grid, V/m, scaled to 1 V
/// on the signal (the mode over V = 2P/I*, so that Z₀ = V/I is [`RfMode::impedance`]). Each cell's
/// E_x is the mean of its bottom and top edges' (where the Yee lattice holds it), E_y of its left
/// and right edges', E_z of its four corners'. At low frequency its transverse part is the static
/// field of the same problem at 1 V on the signal, the grounds at 0.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct RfField {
    /// The mode's numbers.
    pub mode: RfMode,
    /// The grid's nodes across, µm.
    pub x: Vec<f64>,
    /// The grid's nodes up, µm.
    pub y: Vec<f64>,
    /// E (across, up, along) at each cell's centre, V/m at 1 V, cell (i, j) at
    /// j·(x.len() − 1) + i.
    pub e: Vec<[c64; 3]>,
}

impl RfField {
    /// E at (x, y), µm: the value of the cell that holds the point, zero outside the grid.
    pub fn at(&self, x: f64, y: f64) -> [c64; 3] {
        let cell = |nodes: &[f64], v: f64| {
            (nodes[0]..nodes[nodes.len() - 1])
                .contains(&v)
                .then(|| nodes.partition_point(|&n| n <= v) - 1)
        };
        match (cell(&self.x, x), cell(&self.y, y)) {
            (Some(i), Some(j)) => self.e[j * (self.x.len() - 1) + i],
            _ => [c64::new(0.0, 0.0); 3],
        }
    }
}

/// A cell's permittivity from its media's (with their shares) and the interface's normal: per
/// axis across and up, 1 / (n_a² / harmonic + (1 − n_a²) / mean) as the static solver
/// averages, along the line the mean (the field along z is tangential to every interface).
fn average(values: &[([c64; 3], f64)], normal: Option<[f64; 2]>) -> [c64; 3] {
    if let [(t, _)] = values {
        return *t;
    }
    let total: f64 = values.iter().map(|v| v.1).sum();
    let mean = |a: usize| values.iter().map(|(t, f)| t[a] * *f).sum::<c64>() / total;
    let harmonic = |a: usize| total / values.iter().map(|(t, f)| *f / t[a]).sum::<c64>();
    let axis = |a: usize, n: f64| -> c64 {
        match normal {
            Some(_) => 1.0 / (n * n / harmonic(a) + (1.0 - n * n) / mean(a)),
            None => mean(a),
        }
    };
    let [nx, ny] = normal.unwrap_or([0.0, 0.0]);
    [axis(0, nx), axis(1, ny), mean(2)]
}

/// A line's full-wave mode at one frequency ([`FullWave::solve`]), in the e^(jωt) convention.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct RfMode {
    /// The frequency, Hz.
    pub frequency_hz: f64,
    /// γ = α + jβ, 1/m.
    pub gamma: c64,
    /// Z₀ = 2P/|I|², Ω.
    pub impedance: c64,
    /// The power the metal takes over twice the power carried, Np/m.
    pub conductor: f64,
    /// The power the dielectrics take over twice the power carried, Np/m.
    pub dielectric: f64,
    /// |I|² over ∫|H_t|² dA in the window: what picked the mode.
    pub current_per_field: f64,
}

impl RfMode {
    /// The attenuation α = Re γ, Np/m.
    pub fn attenuation(&self) -> f64 {
        self.gamma.re
    }

    /// The attenuation in dB/cm.
    pub fn db_per_cm(&self) -> f64 {
        self.gamma.re * DB_PER_NEPER / 100.0
    }

    /// The RF index βc/ω.
    pub fn index(&self) -> f64 {
        self.gamma.im * SPEED_OF_LIGHT / (2.0 * std::f64::consts::PI * self.frequency_hz)
    }

    /// The loss that is neither the metal's nor the dielectrics': the power radiated into the
    /// open sides' layers, Np/m.
    pub fn radiation(&self) -> f64 {
        self.gamma.re - self.conductor - self.dielectric
    }

    /// The series impedance γZ₀ = R + jωL, Ω/m.
    pub fn series(&self) -> c64 {
        self.gamma * self.impedance
    }

    /// The shunt admittance γ/Z₀ = G + jωC, S/m.
    pub fn shunt(&self) -> c64 {
        self.gamma / self.impedance
    }
}

#[cfg(test)]
#[path = "full_wave_tests.rs"]
mod tests;
