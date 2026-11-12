//! A level set on a design region: φ at the corners of its cells, the solid where φ > 0, each
//! cell's permittivity its solid share's mean of the two (the fill), so that the permittivities
//! are a smooth function of the level set and every node's derivative comes from one adjoint
//! solve's per-cell gradient (Lalau-Keraly et al. 2013's shape derivative, on the cells).
//!
//! **The fill.** Each cell is cut into four triangles about its centre, φ there the mean of its
//! four corners; on each triangle φ is linear, and its share of the triangle where φ > 0 is, with
//! one corner a of the odd sign, a²/((a − b)(a − c)) (or one less it): exact for a φ linear over
//! the cell, continuous with its first derivatives as a corner's value changes sign.
//!
//! C. M. Lalau-Keraly, S. Bhargava, O. D. Miller, E. Yablonovitch, "Adjoint shape optimization
//! applied to electromagnetic design", Opt. Express 21, 21693 (2013), doi:10.1364/OE.21.021693.

use crate::{Error, Result};

/// A level set on `cells` cells of `pixel_um`, the two permittivities it interpolates.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelSet {
    cells: [usize; 2],
    pixel_um: f64,
    eps: [f64; 2],
}

/// The share of a triangle where φ > 0, from φ at its corners, and its derivatives.
fn triangle(v: [f64; 3]) -> (f64, [f64; 3]) {
    let positive = v.iter().filter(|x| **x > 0.0).count();
    match positive {
        0 => (0.0, [0.0; 3]),
        3 => (1.0, [0.0; 3]),
        _ => {
            // the corner of the odd sign: the positive one of one, the other of two
            let odd = if positive == 1 {
                v.iter().position(|x| *x > 0.0)
            } else {
                v.iter().position(|x| *x <= 0.0)
            }
            .expect("a corner");
            let (q, r) = ((odd + 1) % 3, (odd + 2) % 3);
            let (a, b, c) = (v[odd], v[q], v[r]);
            let (ab, ac) = (a - b, a - c);
            let s = a * a / (ab * ac);
            let ds_a = 2.0 * a / (ab * ac) - a * a * (ab + ac) / (ab * ab * ac * ac);
            let ds_b = a * a / (ab * ab * ac);
            let ds_c = a * a / (ab * ac * ac);
            let mut d = [0.0; 3];
            d[odd] = ds_a;
            d[q] = ds_b;
            d[r] = ds_c;
            if positive == 1 {
                (s, d)
            } else {
                (1.0 - s, d.map(|x| -x))
            }
        }
    }
}

/// A cell's share where φ > 0 from its corners (lower left, lower right, upper right, upper
/// left), and its derivatives in them.
pub(crate) fn cell_fill(corners: [f64; 4]) -> (f64, [f64; 4]) {
    let m = 0.25 * corners.iter().sum::<f64>();
    let mut fill = 0.0;
    let mut d = [0.0; 4];
    for k in 0..4 {
        let (s, ds) = triangle([corners[k], corners[(k + 1) % 4], m]);
        fill += 0.25 * s;
        d[k] += 0.25 * ds[0];
        d[(k + 1) % 4] += 0.25 * ds[1];
        for x in &mut d {
            *x += 0.25 * ds[2] * 0.25;
        }
    }
    (fill, d)
}

impl LevelSet {
    /// A level set over `cells` cells (along x and y) each `pixel_um` wide, the cladding's
    /// permittivity `eps[0]` where φ ≤ 0 and the core's `eps[1]` where φ > 0.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no cells, a pixel that isn't positive and finite, or a
    /// permittivity that isn't finite.
    pub fn new(cells: [usize; 2], pixel_um: f64, eps: [f64; 2]) -> Result<LevelSet> {
        if cells.contains(&0) || !(pixel_um.is_finite() && pixel_um > 0.0) {
            return Err(Error::invalid(
                "level set",
                format!("needs cells and a positive pixel, got {cells:?} of {pixel_um} µm"),
            ));
        }
        if !eps.iter().all(|e| e.is_finite()) {
            return Err(Error::invalid(
                "level set",
                format!("the permittivities must be finite, got {eps:?}"),
            ));
        }
        Ok(LevelSet {
            cells,
            pixel_um,
            eps,
        })
    }

    /// The cells along x and y.
    pub fn cells(&self) -> [usize; 2] {
        self.cells
    }

    /// The number of nodes, (nx + 1)(ny + 1): one φ each, i fastest.
    pub fn nodes(&self) -> usize {
        (self.cells[0] + 1) * (self.cells[1] + 1)
    }

    /// A node's place from the region's lower-left corner, µm.
    pub fn node_um(&self, k: usize) -> [f64; 2] {
        let n = self.cells[0] + 1;
        [
            (k % n) as f64 * self.pixel_um,
            (k / n) as f64 * self.pixel_um,
        ]
    }

    fn check(&self, phi: &[f64]) -> Result<()> {
        if phi.len() != self.nodes() {
            return Err(Error::invalid(
                "level set",
                format!("expected {} node values, got {}", self.nodes(), phi.len()),
            ));
        }
        Ok(())
    }

    /// The corners' node indices of cell (i, j), counterclockwise from the lower left.
    fn corners(&self, i: usize, j: usize) -> [usize; 4] {
        let n = self.cells[0] + 1;
        [
            j * n + i,
            j * n + i + 1,
            (j + 1) * n + i + 1,
            (j + 1) * n + i,
        ]
    }

    /// Each cell's share where φ > 0, i fastest.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a φ that isn't one value per node.
    pub fn fill(&self, phi: &[f64]) -> Result<Vec<f64>> {
        self.check(phi)?;
        let [nx, ny] = self.cells;
        Ok((0..nx * ny)
            .map(|p| cell_fill(self.corners(p % nx, p / nx).map(|k| phi[k])).0)
            .collect())
    }

    /// Each cell's permittivity, ε₀ + fill (ε₁ − ε₀).
    ///
    /// # Errors
    ///
    /// As [`LevelSet::fill`].
    pub fn permittivity(&self, phi: &[f64]) -> Result<Vec<f64>> {
        let [e0, e1] = self.eps;
        Ok(self
            .fill(phi)?
            .into_iter()
            .map(|f| e0 + f * (e1 - e0))
            .collect())
    }

    /// ∂F/∂φ at every node from `d_fill`, ∂F/∂(fill) at every cell.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for values that aren't one per node and one per cell.
    pub fn pullback(&self, phi: &[f64], d_fill: &[f64]) -> Result<Vec<f64>> {
        self.check(phi)?;
        let [nx, ny] = self.cells;
        if d_fill.len() != nx * ny {
            return Err(Error::invalid(
                "level set",
                format!("expected {} cell values, got {}", nx * ny, d_fill.len()),
            ));
        }
        let mut out = vec![0.0; self.nodes()];
        for (p, &g) in d_fill.iter().enumerate() {
            if g == 0.0 {
                continue;
            }
            let corners = self.corners(p % nx, p / nx);
            let (_, d) = cell_fill(corners.map(|k| phi[k]));
            for (k, dk) in corners.iter().zip(d) {
                out[*k] += g * dk;
            }
        }
        Ok(out)
    }

    /// ∂F/∂φ from `d_eps`, ∂F/∂ε at every cell (2D FDFD's `mode_power_gradient` restricted to
    /// the region): [`LevelSet::pullback`] of (ε₁ − ε₀) ∂F/∂ε.
    ///
    /// # Errors
    ///
    /// As [`LevelSet::pullback`].
    pub fn gradient(&self, phi: &[f64], d_eps: &[f64]) -> Result<Vec<f64>> {
        let scale = self.eps[1] - self.eps[0];
        let d: Vec<f64> = d_eps.iter().map(|g| g * scale).collect();
        self.pullback(phi, &d)
    }
}

/// A value and its derivatives in a node's φ and its differences: φ, φ_x, φ_y, φ_xx, φ_yy and
/// φ_xy, in that order.
#[derive(Clone, Copy, Debug)]
struct Dual {
    v: f64,
    d: [f64; 6],
}

impl Dual {
    fn constant(v: f64) -> Dual {
        Dual { v, d: [0.0; 6] }
    }

    fn variable(v: f64, k: usize) -> Dual {
        let mut d = [0.0; 6];
        d[k] = 1.0;
        Dual { v, d }
    }

    fn map(self, v: f64, slope: f64) -> Dual {
        Dual {
            v,
            d: self.d.map(|x| slope * x),
        }
    }

    fn sqrt(self) -> Dual {
        let s = self.v.sqrt();
        self.map(s, 0.5 / s)
    }

    fn abs(self) -> Dual {
        self.map(self.v.abs(), if self.v < 0.0 { -1.0 } else { 1.0 })
    }

    /// max(x, 0), the paper's ramp R.
    fn ramp(self) -> Dual {
        if self.v > 0.0 {
            self
        } else {
            Dual::constant(0.0)
        }
    }

    /// atan2(self, x): the angle of (x, self).
    fn atan2(self, x: Dual) -> Dual {
        let r2 = x.v * x.v + self.v * self.v;
        let mut d = [0.0; 6];
        for (k, dk) in d.iter_mut().enumerate() {
            *dk = (x.v * self.d[k] - self.v * x.d[k]) / r2;
        }
        Dual {
            v: self.v.atan2(x.v),
            d,
        }
    }
}

impl std::ops::Add for Dual {
    type Output = Dual;
    fn add(self, o: Dual) -> Dual {
        let mut d = self.d;
        for (a, b) in d.iter_mut().zip(o.d) {
            *a += b;
        }
        Dual { v: self.v + o.v, d }
    }
}

impl std::ops::Sub for Dual {
    type Output = Dual;
    fn sub(self, o: Dual) -> Dual {
        let mut d = self.d;
        for (a, b) in d.iter_mut().zip(o.d) {
            *a -= b;
        }
        Dual { v: self.v - o.v, d }
    }
}

impl std::ops::Mul for Dual {
    type Output = Dual;
    fn mul(self, o: Dual) -> Dual {
        let mut d = [0.0; 6];
        for (k, dk) in d.iter_mut().enumerate() {
            *dk = self.d[k] * o.v + self.v * o.d[k];
        }
        Dual { v: self.v * o.v, d }
    }
}

impl std::ops::Div for Dual {
    type Output = Dual;
    fn div(self, o: Dual) -> Dual {
        let mut d = [0.0; 6];
        for (k, dk) in d.iter_mut().enumerate() {
            *dk = (self.d[k] * o.v - self.v * o.d[k]) / (o.v * o.v);
        }
        Dual { v: self.v / o.v, d }
    }
}

/// Vercruysse et al. 2019's fabrication penalty on a level set (their Eqs. 1–3): a gap term that
/// keeps the second derivative of φ along its gradient within what a grating of feature size d
/// allows, and a curvature term that keeps the boundary's radius of curvature above d/2, each a
/// ramp R(x) = max(x, 0) integrated over the region:
///
/// f = ∬ R(|φ_vv| / ((π/d)|φ| + β φ_v) − π/d) + ζ ∬ R(|κ| arctan((d/π) φ_v/|φ|) − π/d),
///
/// φ_v = |∇φ| and φ_vv = n·H n (n = ∇φ/|∇φ|) the first and second derivatives along the
/// gradient, κ = ∇·n the curvature, β = ⅓ and ζ = 2 theirs. The paper's arctan(φ_v/φ) takes a
/// length (φ_v/φ is per unit length); here it is in units of π/d, the scale Eq. 1 uses, which
/// keeps the paper's limits: π/2 on the boundary (so r ≥ d/2 there) and 0 at an extremum of φ.
/// The derivatives are centred differences at the nodes inside the region's edge, each node's
/// term weighted by a cell's area.
///
/// M. Vercruysse, L. Su, R. Trivedi, N. V. Sapra, J. Vučković, "Analytical level set fabrication
/// constraints for inverse design", Sci. Rep. 9, 8999 (2019), doi:10.1038/s41598-019-45026-0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LevelSetPenalty {
    feature_um: f64,
    beta: f64,
    zeta: f64,
}

impl LevelSetPenalty {
    /// The penalty for a minimum feature size `feature_um`, with their β = ⅓ and ζ = 2.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a feature size that isn't positive and finite.
    pub fn new(feature_um: f64) -> Result<LevelSetPenalty> {
        if !(feature_um.is_finite() && feature_um > 0.0) {
            return Err(Error::invalid(
                "level set penalty",
                format!("the feature size must be positive, got {feature_um}"),
            ));
        }
        Ok(LevelSetPenalty {
            feature_um,
            beta: 1.0 / 3.0,
            zeta: 2.0,
        })
    }

    /// The minimum feature size d, µm.
    pub fn feature_um(&self) -> f64 {
        self.feature_um
    }

    /// The node (i, j)'s gap and curvature terms, each with its derivatives in the node's φ and
    /// differences; none where the gradient vanishes (no direction to take them along).
    fn node(&self, values: [f64; 6]) -> Option<[Dual; 2]> {
        let v: [Dual; 6] = std::array::from_fn(|k| Dual::variable(values[k], k));
        let [phi, px, py, pxx, pyy, pxy] = v;
        let g2 = px * px + py * py;
        if g2.v <= 0.0 {
            return None;
        }
        let g = g2.sqrt();
        let two = Dual::constant(2.0);
        let k = Dual::constant(std::f64::consts::PI / self.feature_um);
        let along = (px * px * pxx + two * px * py * pxy + py * py * pyy) / g2;
        let curvature = (pxx * py * py - two * px * py * pxy + pyy * px * px) / (g2 * g);
        let gap = (along.abs() / (k * phi.abs() + Dual::constant(self.beta) * g) - k).ramp();
        let bend = (curvature.abs() * g.atan2(k * phi.abs()) - k).ramp();
        Some([gap, bend])
    }

    /// The gap and curvature integrals of `phi` on `level`'s nodes, µm.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a φ that isn't one value per node.
    pub fn terms(&self, level: &LevelSet, phi: &[f64]) -> Result<[f64; 2]> {
        let mut out = [0.0; 2];
        self.walk(level, phi, |terms, _| {
            out[0] += terms[0].v;
            out[1] += terms[1].v;
        })?;
        let area = level.pixel_um * level.pixel_um;
        Ok(out.map(|x| x * area))
    }

    /// The penalty, gap + ζ curvature, and its gradient in every node's φ.
    ///
    /// # Errors
    ///
    /// As [`LevelSetPenalty::terms`].
    pub fn evaluate(&self, level: &LevelSet, phi: &[f64]) -> Result<(f64, Vec<f64>)> {
        let mut value = 0.0;
        let mut gradient = vec![0.0; level.nodes()];
        let area = level.pixel_um * level.pixel_um;
        let h = level.pixel_um;
        let n = level.cells[0] + 1;
        let zeta = self.zeta;
        self.walk(level, phi, |terms, p| {
            let t = terms[0] + Dual::constant(zeta) * terms[1];
            value += t.v * area;
            let d = t.d.map(|x| x * area);
            // the stencils: φ, φ_x, φ_y, φ_xx, φ_yy, φ_xy
            gradient[p] += d[0] - 2.0 * d[3] / (h * h) - 2.0 * d[4] / (h * h);
            gradient[p + 1] += d[1] / (2.0 * h) + d[3] / (h * h);
            gradient[p - 1] += -d[1] / (2.0 * h) + d[3] / (h * h);
            gradient[p + n] += d[2] / (2.0 * h) + d[4] / (h * h);
            gradient[p - n] += -d[2] / (2.0 * h) + d[4] / (h * h);
            let c = d[5] / (4.0 * h * h);
            gradient[p + n + 1] += c;
            gradient[p - n - 1] += c;
            gradient[p + n - 1] -= c;
            gradient[p - n + 1] -= c;
        })?;
        Ok((value, gradient))
    }

    /// Calls `each` with every inner node's two terms and its index.
    fn walk(
        &self,
        level: &LevelSet,
        phi: &[f64],
        mut each: impl FnMut([Dual; 2], usize),
    ) -> Result<()> {
        level.check(phi)?;
        let (n, m) = (level.cells[0] + 1, level.cells[1] + 1);
        let h = level.pixel_um;
        for j in 1..m.saturating_sub(1) {
            for i in 1..n - 1 {
                let p = j * n + i;
                let f = |di: isize, dj: isize| phi[(p as isize + dj * n as isize + di) as usize];
                let values = [
                    f(0, 0),
                    (f(1, 0) - f(-1, 0)) / (2.0 * h),
                    (f(0, 1) - f(0, -1)) / (2.0 * h),
                    (f(1, 0) - 2.0 * f(0, 0) + f(-1, 0)) / (h * h),
                    (f(0, 1) - 2.0 * f(0, 0) + f(0, -1)) / (h * h),
                    (f(1, 1) - f(1, -1) - f(-1, 1) + f(-1, -1)) / (4.0 * h * h),
                ];
                if let Some(terms) = self.node(values) {
                    each(terms, p);
                }
            }
        }
        Ok(())
    }
}
