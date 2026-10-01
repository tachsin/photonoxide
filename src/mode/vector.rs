//! Full-vector modes of a waveguide cross-section, by finite differences.
//!
//! A. B. Fallahkhair, K. S. Li, T. E. Murphy, "Vector finite difference modesolver for
//! anisotropic dielectric waveguides", J. Lightwave Technol. 26, 1423 (2008),
//! [doi:10.1109/JLT.2008.923643](https://doi.org/10.1109/JLT.2008.923643). The unknowns are
//! the transverse magnetic field components H_x and H_y at the nodes of a rectilinear grid
//! (spacing need not be uniform); the permittivity is uniform in each cell between nodes, a
//! tensor with ε_xx, ε_xy, ε_yx, ε_yy and ε_zz (the waveguide symmetric under z → −z). The
//! coupled equations (4a)–(4b) are discretized with the coefficients of the paper's Appendix,
//! Eqs. (21)–(36), and their eigenvalues are β² (Eq. (8)). The fields outside the window are
//! zero.
//!
//! The paper uses the e^(jωt) convention; photonoxide uses e^(−iωt) ([`crate::units`]). The
//! eigenvalue equations hold in either, written with the permittivity of the convention in use:
//! a lossy medium is +i here, and a gyrotropic tensor written with +jΔ in the paper is −iΔ here.
//! The eigenvalue is β², and the mode travels as e^(i(βz − ωt)), so a lossy mode has Im β > 0.
//!
//! One misprint is corrected: Eq. (31) divides by "v₁₂", which the paper never defines; it is
//! v₂₁, as in the matching term of Eq. (30) under the mirror x → −x (tested: a structure and its
//! mirror image have the same modes).

use faer::c64;

use crate::units::Wavelength;
use crate::{Error, Result};

/// The relative permittivity of a cell: the transverse tensor and ε_zz.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Permittivity {
    /// ε_xx.
    pub xx: c64,
    /// ε_xy.
    pub xy: c64,
    /// ε_yx.
    pub yx: c64,
    /// ε_yy.
    pub yy: c64,
    /// ε_zz.
    pub zz: c64,
}

impl Permittivity {
    /// An isotropic medium of relative permittivity ε.
    pub fn isotropic(eps: c64) -> Permittivity {
        let zero = c64::new(0.0, 0.0);
        Permittivity {
            xx: eps,
            xy: zero,
            yx: zero,
            yy: eps,
            zz: eps,
        }
    }

    /// The tensor with x and y exchanged (Eq. (36)).
    fn transposed(self) -> Permittivity {
        Permittivity {
            xx: self.yy,
            xy: self.yx,
            yx: self.xy,
            yy: self.xx,
            zz: self.zz,
        }
    }
}

/// A rectilinear grid over the cross-section and the permittivity of each of its cells.
#[derive(Clone, Debug, PartialEq)]
pub struct CrossSection {
    x: Vec<f64>,
    y: Vec<f64>,
    cells: Vec<Permittivity>,
}

impl CrossSection {
    /// Nodes at `x` and `y` (µm, strictly increasing, at least 3 each), and the permittivity of
    /// each cell between them: cell (i, j), between `x[i]` and `x[i+1]` and `y[j]` and `y[j+1]`, is
    /// `cells[i * (y.len() − 1) + j]`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for fewer than 3 nodes along a side, coordinates that aren't
    /// finite and strictly increasing, the wrong number of cells, or a non-finite permittivity.
    pub fn new(x: Vec<f64>, y: Vec<f64>, cells: Vec<Permittivity>) -> Result<CrossSection> {
        for (name, v) in [("x", &x), ("y", &y)] {
            if v.len() < 3 {
                return Err(Error::invalid(
                    "cross-section",
                    format!("needs at least 3 nodes along {name}, got {}", v.len()),
                ));
            }
            if !v.iter().all(|c| c.is_finite()) || v.windows(2).any(|w| w[1] <= w[0]) {
                return Err(Error::invalid(
                    "cross-section",
                    format!("the {name} nodes must be finite and strictly increasing"),
                ));
            }
        }
        let expected = (x.len() - 1) * (y.len() - 1);
        if cells.len() != expected {
            return Err(Error::invalid(
                "cross-section",
                format!("needs {expected} cells, got {}", cells.len()),
            ));
        }
        let finite = |e: &Permittivity| {
            [e.xx, e.xy, e.yx, e.yy, e.zz]
                .iter()
                .all(|v| v.re.is_finite() && v.im.is_finite())
        };
        if !cells.iter().all(finite) {
            return Err(Error::invalid(
                "cross-section",
                "every permittivity must be finite",
            ));
        }
        Ok(CrossSection { x, y, cells })
    }

    /// A uniform grid of `nx` × `ny` cells over [x0, x1] × [y0, y1] (µm), each cell taking the
    /// permittivity `eps` gives at its centre.
    ///
    /// # Errors
    ///
    /// As [`CrossSection::new`].
    pub fn uniform(
        (x0, x1, nx): (f64, f64, usize),
        (y0, y1, ny): (f64, f64, usize),
        eps: impl Fn(f64, f64) -> Permittivity,
    ) -> Result<CrossSection> {
        let line = |a: f64, b: f64, n: usize| -> Vec<f64> {
            (0..=n).map(|i| a + (b - a) * i as f64 / n as f64).collect()
        };
        let (x, y) = (line(x0, x1, nx), line(y0, y1, ny));
        let mut cells = Vec::with_capacity(nx * ny);
        for i in 0..nx {
            for j in 0..ny {
                cells.push(eps(0.5 * (x[i] + x[i + 1]), 0.5 * (y[j] + y[j + 1])));
            }
        }
        CrossSection::new(x, y, cells)
    }

    /// The node coordinates along x, µm.
    pub fn x(&self) -> &[f64] {
        &self.x
    }

    /// The node coordinates along y, µm.
    pub fn y(&self) -> &[f64] {
        &self.y
    }

    /// The cell (i, j), clamped to the grid: outside the window the edge cells continue.
    fn cell(&self, i: isize, j: isize) -> Permittivity {
        let (nx, ny) = (self.x.len() - 1, self.y.len() - 1);
        let i = i.clamp(0, nx as isize - 1) as usize;
        let j = j.clamp(0, ny as isize - 1) as usize;
        self.cells[i * ny + j]
    }
}

/// The nine coefficients of a node's equation: the node itself and its neighbours.
#[derive(Clone, Copy, Debug, Default)]
struct Stencil {
    p: c64,
    n: c64,
    s: c64,
    e: c64,
    w: c64,
    ne: c64,
    se: c64,
    sw: c64,
    nw: c64,
}

/// A_xx and A_xy at a node, Eqs. (21)–(35): regions 1 (NW), 2 (SW), 3 (SE), 4 (NE) around it
/// (Fig. 1), distances n, s, e, w to its neighbours, and k².
#[allow(clippy::too_many_arguments)]
fn coefficients(
    r1: Permittivity,
    r2: Permittivity,
    r3: Permittivity,
    r4: Permittivity,
    n: c64,
    s: c64,
    e: c64,
    w: c64,
    k2: f64,
) -> (Stencil, Stencil) {
    let one = c64::new(1.0, 0.0);
    let two = c64::new(2.0, 0.0);
    let (yy1, yy2, yy3, yy4) = (r1.yy, r2.yy, r3.yy, r4.yy);
    let (yx1, yx2, yx3, yx4) = (r1.yx, r2.yx, r3.yx, r4.yx);
    let (zz1, zz2, zz3, zz4) = (r1.zz, r2.zz, r3.zz, r4.zz);
    let v21 = n * yy2 + s * yy1;
    let v34 = n * yy3 + s * yy4;
    let ew = e + w;

    let mut xx = Stencil {
        // (21)
        n: (yy3 / zz4) * (two * e * yy4 - n * yx4) / (v34 * n * ew)
            + (yy2 / zz1) * (two * w * yy1 + n * yx1) / (v21 * n * ew),
        // (22)
        s: (yy4 / zz3) * (two * e * yy3 - s * yx3) / (v34 * s * ew)
            + (yy1 / zz2) * (two * w * yy2 + s * yx2) / (v21 * s * ew),
        // (23)
        e: two / (e * ew) + (yy4 * yx3 / zz3 - yy3 * yx4 / zz4) / (v34 * ew),
        // (24)
        w: two / (w * ew) + (yy2 * yx1 / zz1 - yy1 * yx2 / zz2) / (v21 * ew),
        // (25)
        ne: yx4 * yy3 / (zz4 * v34 * ew),
        se: -yx3 * yy4 / (zz3 * v34 * ew),
        // (26)
        sw: yx2 * yy1 / (zz2 * v21 * ew),
        nw: -yx1 * yy2 / (zz1 * v21 * ew),
        p: c64::new(0.0, 0.0),
    };
    // (27)
    xx.p = -(xx.n + xx.s + xx.e + xx.w + xx.ne + xx.se + xx.sw + xx.nw)
        + k2 * (n + s) / ew * (yy4 * yy3 * e / v34 + yy1 * yy2 * w / v21);

    let mut xy = Stencil {
        // (28)
        n: (s * yy2 * yy4 / (v21 * v34) - s * yy1 * yy3 / (v21 * v34) + yy3 * yy4 / (zz4 * v34)
            - yy2 * yy1 / (zz1 * v21))
            / ew,
        // (29)
        s: (n * yy2 * yy4 / (v21 * v34) - n * yy1 * yy3 / (v21 * v34) + yy1 * yy2 / (zz2 * v21)
            - yy4 * yy3 / (zz3 * v34))
            / ew,
        // (30)
        e: (yy4 * (one - yy3 / zz3) - yy3 * (one - yy4 / zz4)) / (v34 * ew)
            - two / (e * ew * ew)
                * (n * w * yx1 * yy2 / (zz1 * v21)
                    + s * w * yx2 * yy1 / (zz2 * v21)
                    + n * e * yx4 * yy3 / (zz4 * v34)
                    + s * e * yx3 * yy4 / (zz3 * v34)
                    + w * w * yy1 * yy2 / v21 * (one / zz1 - one / zz2)
                    + e * w * yy3 * yy4 / v34 * (one / zz4 - one / zz3)),
        // (31), with the paper's "v12" read as v21
        w: (yy2 * (one - yy1 / zz1) - yy1 * (one - yy2 / zz2)) / (v21 * ew)
            - two / (w * ew * ew)
                * (n * e * yx4 * yy3 / (zz4 * v34)
                    + s * e * yx3 * yy4 / (zz3 * v34)
                    + n * w * yx1 * yy2 / (zz1 * v21)
                    + s * w * yx2 * yy1 / (zz2 * v21)
                    + e * e * yy4 * yy3 / v34 * (one / zz3 - one / zz4)
                    + e * w * yy2 * yy1 / v21 * (one / zz2 - one / zz1)),
        // (32)
        ne: (yy3 - yy3 * yy4 / zz4) / (v34 * ew),
        se: (yy4 * yy3 / zz3 - yy4) / (v34 * ew),
        // (33)
        nw: (yy2 * yy1 / zz1 - yy2) / (v21 * ew),
        sw: (yy1 - yy1 * yy2 / zz2) / (v21 * ew),
        p: c64::new(0.0, 0.0),
    };
    // (34)
    xy.p = -(xy.n + xy.s + xy.e + xy.w + xy.ne + xy.se + xy.sw + xy.nw)
        - k2 * (w * (n * yx1 * yy2 + s * yx2 * yy1) / (v21 * ew)
            + e * (s * yx3 * yy4 + n * yx4 * yy3) / (v34 * ew));
    (xx, xy)
}

/// The matrix of Eq. (8), as (row, column, value) entries over the unknowns
/// [H_x at every node; H_y at every node], node (i, j) being i·(ny + 1) + j.
fn assemble(cs: &CrossSection, k2: f64) -> (usize, Vec<(usize, usize, c64)>) {
    let (nxn, nyn) = (cs.x.len(), cs.y.len());
    let nodes = nxn * nyn;
    let node = |i: isize, j: isize| -> Option<usize> {
        (i >= 0 && j >= 0 && (i as usize) < nxn && (j as usize) < nyn)
            .then(|| i as usize * nyn + j as usize)
    };
    let spacing = |v: &[f64], i: usize, ahead: bool| -> f64 {
        // at the window's edge the spacing continues, the field beyond being zero
        let last = v.len() - 1;
        match (ahead, i) {
            (true, i) if i < last => v[i + 1] - v[i],
            (true, _) => v[last] - v[last - 1],
            (false, 0) => v[1] - v[0],
            (false, i) => v[i] - v[i - 1],
        }
    };
    let mut entries = Vec::with_capacity(nodes * 36);
    for i in 0..nxn {
        for j in 0..nyn {
            let (ii, jj) = (i as isize, j as isize);
            let r1 = cs.cell(ii - 1, jj);
            let r2 = cs.cell(ii - 1, jj - 1);
            let r3 = cs.cell(ii, jj - 1);
            let r4 = cs.cell(ii, jj);
            let c = |v: f64| c64::new(v, 0.0);
            let (n, s) = (c(spacing(&cs.y, j, true)), c(spacing(&cs.y, j, false)));
            let (e, w) = (c(spacing(&cs.x, i, true)), c(spacing(&cs.x, i, false)));
            let (axx, axy) = coefficients(r1, r2, r3, r4, n, s, e, w, k2);
            // Eq. (36): x ↔ y, so n ↔ e, s ↔ w, region 1 ↔ 3, and the tensor transposed; the
            // results are A_yy and A_yx with their neighbours mirrored back (N ↔ E, S ↔ W,
            // NW ↔ SE)
            let (ayy, ayx) = coefficients(
                r3.transposed(),
                r2.transposed(),
                r1.transposed(),
                r4.transposed(),
                e,
                w,
                n,
                s,
                k2,
            );
            let p = node(ii, jj).unwrap_or(0);
            let around = |st: &Stencil, mirrored: bool| -> [(isize, isize, c64); 9] {
                if mirrored {
                    [
                        (0, 0, st.p),
                        (1, 0, st.n),
                        (-1, 0, st.s),
                        (0, 1, st.e),
                        (0, -1, st.w),
                        (1, 1, st.ne),
                        (-1, 1, st.se),
                        (-1, -1, st.sw),
                        (1, -1, st.nw),
                    ]
                } else {
                    [
                        (0, 0, st.p),
                        (0, 1, st.n),
                        (0, -1, st.s),
                        (1, 0, st.e),
                        (-1, 0, st.w),
                        (1, 1, st.ne),
                        (1, -1, st.se),
                        (-1, -1, st.sw),
                        (-1, 1, st.nw),
                    ]
                }
            };
            for (row, stencils) in [
                (p, [(&axx, 0, false), (&axy, nodes, false)]),
                (nodes + p, [(&ayx, 0, true), (&ayy, nodes, true)]),
            ] {
                for (st, offset, mirrored) in stencils {
                    for (di, dj, v) in around(st, mirrored) {
                        if v.re == 0.0 && v.im == 0.0 {
                            continue;
                        }
                        if let Some(q) = node(ii + di, jj + dj) {
                            entries.push((row, offset + q, v));
                        }
                    }
                }
            }
        }
    }
    (2 * nodes, entries)
}

/// A full-vector mode of a cross-section.
#[derive(Clone, Debug, PartialEq)]
pub struct VectorMode {
    beta2: c64,
    k: f64,
    nyn: usize,
    hx: Vec<c64>,
    hy: Vec<c64>,
}

impl VectorMode {
    /// The effective index n_eff = β/k of the forward-travelling mode (Re β ≥ 0, the principal
    /// root of β²); in a passive structure its imaginary part is ≥ 0, the mode decaying along z.
    pub fn effective_index(&self) -> c64 {
        self.beta2.sqrt() / self.k
    }

    /// H_x at node (i, j), normalized so the largest transverse component is 1.
    pub fn hx(&self, i: usize, j: usize) -> c64 {
        self.hx[i * self.nyn + j]
    }

    /// H_y at node (i, j), on the same scale as [`VectorMode::hx`].
    pub fn hy(&self, i: usize, j: usize) -> c64 {
        self.hy[i * self.nyn + j]
    }

    /// The share of the transverse magnetic field in H_y: near 1 for a TE-like mode (E mostly
    /// along x), near 0 for a TM-like one.
    pub fn te_fraction(&self) -> f64 {
        let x: f64 = self.hx.iter().map(|v| v.norm_sqr()).sum();
        let y: f64 = self.hy.iter().map(|v| v.norm_sqr()).sum();
        y / (x + y)
    }
}

/// The `count` modes of `cs` at `wavelength` with effective indices nearest `near` (the
/// highest index in the cross-section when `None`: the fundamental modes), nearest first.
///
/// # Errors
///
/// [`Error::InvalidValue`] for a count of 0, or if the eigenproblem fails to converge.
pub fn modes(
    cs: &CrossSection,
    wavelength: Wavelength,
    count: usize,
    near: Option<f64>,
) -> Result<Vec<VectorMode>> {
    if count == 0 {
        return Err(Error::invalid("mode count", "must be at least 1"));
    }
    let k = wavelength.wavenumber();
    let k2 = k * k;
    let n_max = near.unwrap_or_else(|| {
        cs.cells
            .iter()
            .map(|e| e.xx.re.max(e.yy.re).sqrt())
            .fold(1.0, f64::max)
    });
    let (size, entries) = assemble(cs, k2);
    let shift = c64::new(k2 * n_max * n_max, 0.0);
    let pairs = crate::eigen::nearest(size, &entries, shift, count, 1e-9)?;
    let nodes = size / 2;
    Ok(pairs
        .into_iter()
        .map(|p| {
            let (hx, hy) = p.vector.split_at(nodes);
            let peak = p
                .vector
                .iter()
                .copied()
                .max_by(|a, b| a.norm().total_cmp(&b.norm()))
                .unwrap_or(c64::new(1.0, 0.0));
            VectorMode {
                beta2: p.value,
                k,
                nyn: cs.y.len(),
                hx: hx.iter().map(|v| v / peak).collect(),
                hy: hy.iter().map(|v| v / peak).collect(),
            }
        })
        .collect())
}

/// The book's 500 × 220 nm silicon strip in oxide (3.473 in 1.444), on a uniform grid of
/// spacing `h` (µm) with nodes on the core's edges, in a 2.1 × 1.5 µm window.
pub(crate) fn strip(h: f64) -> CrossSection {
    let nx = (2.1 / h).round() as usize;
    let ny = (1.5 / h).round() as usize;
    CrossSection::uniform((-1.05, 1.05, nx), (-0.75, 0.75, ny), |x, y| {
        let eps = if x.abs() < 0.25 && y.abs() < 0.11 {
            3.473 * 3.473
        } else {
            1.444 * 1.444
        };
        Permittivity::isotropic(c64::new(eps, 0.0))
    })
    .expect("a valid grid")
}

/// The full-vector solver's error on the book's slab (220 nm of 3.473 in 1.444) turned on its
/// side and uniform along y, against the exact slab mode of `polarization`, at spacing `h`
/// (µm) in a 4 µm window. `te_like` picks the vector mode whose H is mostly along y.
pub(crate) fn slab_limit_error(
    polarization: crate::mode::Polarization,
    te_like: bool,
    wavelength: Wavelength,
    h: f64,
) -> f64 {
    let exact = crate::mode::slab::Slab::new(1.444, 3.473, 1.444, crate::units::Length::nm(220.0))
        .expect("a valid slab")
        .modes(polarization, wavelength)[0]
        .effective_index();
    let half = 2.01;
    let nx = (2.0 * half / h).round() as usize;
    let ny = 8usize;
    let width = 0.16;
    let cs = CrossSection::uniform((-half, half, nx), (0.0, width, ny), |x, _| {
        let eps = if x.abs() < 0.11 {
            3.473 * 3.473
        } else {
            1.444 * 1.444
        };
        Permittivity::isotropic(c64::new(eps, 0.0))
    })
    .expect("a valid grid");
    let hy = width / ny as f64;
    let yterm = 4.0 / (hy * hy)
        * (std::f64::consts::PI / (2.0 * (ny as f64 + 2.0)))
            .sin()
            .powi(2);
    let found = modes(&cs, wavelength, 2, None).expect("the slab's modes converge");
    let m = found
        .iter()
        .find(|m| (m.te_fraction() > 0.5) == te_like)
        .expect("one mode of each polarization");
    (m.beta2.re + yterm).sqrt() / wavelength.wavenumber() - exact
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::Polarization;
    use std::f64::consts::PI;

    fn iso(eps: f64) -> Permittivity {
        Permittivity::isotropic(c64::new(eps, 0.0))
    }

    #[test]
    fn a_uniform_box_has_its_exact_discrete_modes() {
        // homogeneous ε, H = 0 one spacing beyond the window: the modes are sines with
        // β² = k²ε − (4/hx²) sin²(pπ/(2(Nx+1))) − (4/hy²) sin²(qπ/(2(Ny+1))), Nx nodes along x
        let (nx, ny) = (20, 14);
        let (lx, ly) = (2.0, 1.4);
        let eps = 2.25;
        let cs = CrossSection::uniform((0.0, lx, nx), (0.0, ly, ny), |_, _| iso(eps)).unwrap();
        let lam = Wavelength::um(1.0).unwrap();
        let k2 = lam.wavenumber().powi(2);
        let (hx, hy) = (lx / nx as f64, ly / ny as f64);
        let (nodes_x, nodes_y) = ((nx + 1) as f64, (ny + 1) as f64);
        let term = |p: f64, h: f64, nodes: f64| {
            4.0 / (h * h) * (p * PI / (2.0 * (nodes + 1.0))).sin().powi(2)
        };
        let found = modes(&cs, lam, 4, None).unwrap();
        let mut expected: Vec<f64> = Vec::new();
        for p in 1..=3 {
            for q in 1..=3 {
                expected.push(k2 * eps - term(p as f64, hx, nodes_x) - term(q as f64, hy, nodes_y));
            }
        }
        expected.sort_by(|a, b| b.total_cmp(a));
        // each discrete eigenvalue appears twice: once for H_x, once for H_y
        let want = [expected[0], expected[0], expected[1], expected[1]];
        for (m, w) in found.iter().zip(want) {
            assert!(
                (m.beta2.re - w).abs() < 1e-9 * w.abs(),
                "{} vs {w}",
                m.beta2
            );
            assert!(m.beta2.im.abs() < 1e-9 * w.abs());
        }
    }

    #[test]
    fn a_uniform_isotropic_region_decouples_hx_and_hy() {
        // A_xy vanishes where the four cells around a node are the same isotropic medium
        let (xx, xy) = coefficients(
            iso(4.0),
            iso(4.0),
            iso(4.0),
            iso(4.0),
            c64::new(0.1, 0.0),
            c64::new(0.2, 0.0),
            c64::new(0.15, 0.0),
            c64::new(0.05, 0.0),
            30.0,
        );
        for v in [xy.p, xy.n, xy.s, xy.e, xy.w, xy.ne, xy.se, xy.sw, xy.nw] {
            assert!(v.norm() < 1e-12, "{v}");
        }
        // and A_xx is the standard second difference plus k² ε
        let (n, s, e, w) = (0.1, 0.2, 0.15, 0.05);
        assert!((xx.n.re - 2.0 / (n * (n + s))).abs() < 1e-9);
        assert!((xx.e.re - 2.0 / (e * (e + w))).abs() < 1e-9);
        let sum =
            2.0 / (n * (n + s)) + 2.0 / (s * (n + s)) + 2.0 / (e * (e + w)) + 2.0 / (w * (e + w));
        assert!((xx.p.re - (30.0 * 4.0 - sum)).abs() < 1e-9);
    }

    /// A waveguide whose core is uniaxial (ε_zz ≠ ε_xx = ε_yy), placed off-centre.
    fn uniaxial(mirror: bool) -> CrossSection {
        CrossSection::uniform((-1.2, 1.2, 48), (-1.0, 1.0, 40), |x, y| {
            let x = if mirror { -x } else { x };
            if (x - 0.15).abs() < 0.3 && y.abs() < 0.2 {
                Permittivity {
                    zz: c64::new(9.0, 0.0),
                    ..iso(11.0)
                }
            } else if y < -0.2 && x < 0.4 {
                iso(2.4)
            } else {
                iso(2.1)
            }
        })
        .unwrap()
    }

    #[test]
    fn a_structure_and_its_mirror_image_have_the_same_modes() {
        // exercises Eqs. (30) and (31) (the ε_zz terms) against each other
        let lam = Wavelength::um(1.55).unwrap();
        let a = modes(&uniaxial(false), lam, 2, None).unwrap();
        let b = modes(&uniaxial(true), lam, 2, None).unwrap();
        for (ma, mb) in a.iter().zip(&b) {
            let (na, nb) = (ma.effective_index(), mb.effective_index());
            assert!((na - nb).norm() < 1e-10, "{na} vs {nb}");
        }
    }

    #[test]
    fn a_strip_has_a_te_like_fundamental_mode() {
        // 500 x 220 nm silicon in oxide at 1550 nm: TE-like first, with H mostly along y
        let cs = strip(0.02);
        let found = modes(&cs, Wavelength::um(1.55).unwrap(), 2, None).unwrap();
        let te = &found[0];
        assert!(te.te_fraction() > 0.9, "{}", te.te_fraction());
        let n = te.effective_index();
        assert!(n.re > 2.3 && n.re < 2.6 && n.im.abs() < 1e-9, "{n}");
        assert!(found[1].te_fraction() < 0.1, "the second mode is TM-like");
    }

    #[test]
    fn the_slab_limit_converges_at_second_order_to_the_exact_slab() {
        // the book's slab (220 nm of 3.473 in 1.444), uniform along y: the y-dependence is an
        // exact discrete sine, so the slab's β² = β² + (4/hy²) sin²(π/(2(Ny+1))) exactly
        let lam = Wavelength::um(1.55).unwrap();
        for (pol, te_like) in [(Polarization::Te, false), (Polarization::Tm, true)] {
            let errors: Vec<f64> = [0.02, 0.01, 0.005]
                .iter()
                .map(|&h| slab_limit_error(pol, te_like, lam, h))
                .collect();
            for w in errors.windows(2) {
                let order = (w[0] / w[1]).abs().log2();
                assert!(
                    (order - 2.0).abs() < 0.1,
                    "{pol:?}: order {order}, errors {errors:?}"
                );
            }
            assert!(errors[2].abs() < 2e-4, "{pol:?}: {errors:?}");
        }
    }

    #[test]
    #[ignore = "a convergence study, run by hand (cargo test --release -- --ignored --nocapture)"]
    fn strip_convergence() {
        // 500 x 220 nm silicon in oxide at 1550 nm: the corners slow convergence to about
        // order 0.6–0.8 (TE 2.447067, 2.445713, 2.444396, 2.443506 at 20, 10, 5, 2.5 nm)
        for h in [0.02f64, 0.01, 0.005, 0.0025] {
            let t = std::time::Instant::now();
            let m = modes(&strip(h), Wavelength::um(1.55).unwrap(), 2, None).unwrap();
            println!(
                "h {:.1} nm: TE {:.6} ({:.3}), TM {:.6} ({:.3})  {:.1} s",
                h * 1000.0,
                m[0].effective_index().re,
                m[0].te_fraction(),
                m[1].effective_index().re,
                m[1].te_fraction(),
                t.elapsed().as_secs_f64()
            );
        }
    }

    #[test]
    fn bad_cross_sections_are_errors() {
        let cell = iso(2.0);
        assert!(CrossSection::new(vec![0.0, 1.0], vec![0.0, 1.0, 2.0], vec![cell; 2]).is_err());
        assert!(
            CrossSection::new(vec![0.0, 1.0, 1.0], vec![0.0, 1.0, 2.0], vec![cell; 4]).is_err()
        );
        assert!(
            CrossSection::new(vec![0.0, 1.0, 2.0], vec![0.0, 1.0, 2.0], vec![cell; 3]).is_err()
        );
        let bad = Permittivity::isotropic(c64::new(f64::NAN, 0.0));
        assert!(CrossSection::new(vec![0.0, 1.0, 2.0], vec![0.0, 1.0, 2.0], vec![bad; 4]).is_err());
    }
}
