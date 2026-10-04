//! Full-vector modes of a waveguide cross-section, by finite differences.
//!
//! A. B. Fallahkhair, K. S. Li, T. E. Murphy, "Vector finite difference modesolver for
//! anisotropic dielectric waveguides", J. Lightwave Technol. 26, 1423 (2008),
//! [doi:10.1109/JLT.2008.923643](https://doi.org/10.1109/JLT.2008.923643). The unknowns are
//! the transverse magnetic field components H_x and H_y at the nodes of a rectilinear grid
//! (spacing need not be uniform); the permittivity is uniform in each cell between nodes, a
//! tensor with ε_xx, ε_xy, ε_yx, ε_yy and ε_zz (the waveguide symmetric under z → −z). The
//! coupled equations (4a)–(4b) are discretized with the coefficients of the paper's Appendix,
//! Eqs. (21)–(36), and their eigenvalues are β² (Eq. (8)). Beyond each edge of the window the
//! field is zero, or a [`Boundary`] wall mirrors it, so a symmetric waveguide can be solved on
//! half or a quarter of its cross-section.
//!
//! The scheme converges at second order where interfaces are straight, and more slowly at
//! dielectric corners, where the fields' derivatives are singular (G. R. Hadley, J. Lightwave
//! Technol. 20, 1219 (2002), [doi:10.1109/JLT.2002.800371](https://doi.org/10.1109/JLT.2002.800371)):
//! on Hadley's four corner problems, at about first order at convex corners (a high-index
//! quadrant) and at 1.4–1.8 at concave ones, as standard finite differences do. Hadley's own
//! equations, on the same unknowns, are [`crate::mode::hadley`]: about second order there.
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

/// What lies at an edge of the window.
///
/// A wall lies on the edge's nodes, and the field beyond it is the mirror image of the field
/// inside: a wall is a plane of symmetry, so a waveguide that is mirror-symmetric can be solved
/// on half (or a quarter) of its cross-section, for the modes of one symmetry. The H component
/// normal to a wall is odd across it under an electric wall and even under a magnetic one; the
/// tangential component the opposite. An odd component is zero on the wall.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Boundary {
    /// The field is zero one spacing beyond the edge's nodes.
    #[default]
    Zero,
    /// A perfect electric conductor: the tangential E vanishes, so the normal H is zero on the
    /// wall and the tangential H has zero normal derivative. For a TE-like mode (E mostly along
    /// x), the vertical plane through a symmetric core.
    ElectricWall,
    /// A perfect magnetic conductor: the tangential H is zero on the wall and the normal H has
    /// zero normal derivative. For a TE-like mode, the horizontal plane through a symmetric core.
    MagneticWall,
}

/// The boundaries on the window's four edges: west (smallest x), east, south (smallest y) and
/// north.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Boundaries {
    /// At the smallest x.
    pub west: Boundary,
    /// At the largest x.
    pub east: Boundary,
    /// At the smallest y.
    pub south: Boundary,
    /// At the largest y.
    pub north: Boundary,
}

/// Perfectly matched layers inside the window's edges: the last `west`, `east`, `south` and
/// `north` micrometres of the window (0 for none) absorb the waves that reach them, so a leaky
/// mode's loss can be found and the window's edges don't reflect.
///
/// W. C. Chew, J. M. Jin, E. Michielssen, "Complex coordinate stretching as a generalized
/// absorbing boundary condition", Microw. Opt. Technol. Lett. 15, 363 (1997),
/// [doi:10.1002/(SICI)1098-2760(19970820)15:6<363::AID-MOP8>3.0.CO;2-C](https://doi.org/10.1002/(SICI)1098-2760(19970820)15:6%3C363::AID-MOP8%3E3.0.CO;2-C),
/// after W. C. Chew, W. H. Weedon, Microw. Opt. Technol. Lett. 7, 599 (1994),
/// [doi:10.1002/mop.4650071304](https://doi.org/10.1002/mop.4650071304). Inside a layer of
/// thickness d the coordinate is stretched by s = 1 + iα(u/d)², u the depth into the layer
/// (Eq. 44), so it becomes complex, x̃ = x ± iα u³/(3d²) (Eq. 45; plus towards larger x). A
/// wave travelling into the layer then decays without reflecting, at any angle, for both
/// polarizations, and also where a dielectric interface runs into it. Fallahkhair et al.'s
/// equations take the complex grid spacings as they are (their ref. 21). Both papers use
/// e^(−iωt), as photonoxide does: the stretched coordinate's imaginary part has the sign of the
/// outward direction. Beyond the layer the field is zero.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pml {
    /// The layer's thickness at the smallest x, µm.
    pub west: f64,
    /// At the largest x.
    pub east: f64,
    /// At the smallest y.
    pub south: f64,
    /// At the largest y.
    pub north: f64,
    /// The stretching's strength α: s = 1 + iα at the window's edge. A few units absorb well
    /// over a layer about a wavelength thick.
    pub strength: f64,
}

/// A field component, H_x or H_y.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Component {
    X,
    Y,
}

impl Component {
    /// The other transverse component.
    pub(crate) fn other(self) -> Component {
        match self {
            Component::X => Component::Y,
            Component::Y => Component::X,
        }
    }
}

/// The sign a component takes across a wall normal to `normal`: −1 if it is odd, 1 if even;
/// `None` for [`Boundary::Zero`].
fn parity(boundary: Boundary, normal: Component, component: Component) -> Option<f64> {
    let is_normal = normal == component;
    match boundary {
        Boundary::Zero => None,
        Boundary::ElectricWall => Some(if is_normal { -1.0 } else { 1.0 }),
        Boundary::MagneticWall => Some(if is_normal { 1.0 } else { -1.0 }),
    }
}

/// A rectilinear grid over the cross-section, the permittivity of each of its cells, and what
/// lies at its edges.
#[derive(Clone, Debug, PartialEq)]
pub struct CrossSection {
    x: Vec<f64>,
    y: Vec<f64>,
    cells: Vec<Permittivity>,
    boundaries: Boundaries,
    pml: Pml,
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
        Ok(CrossSection {
            x,
            y,
            cells,
            boundaries: Boundaries::default(),
            pml: Pml::default(),
        })
    }

    /// The same cross-section with `boundaries` at its edges ([`Boundary::Zero`] on all four
    /// by default).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a wall beside a cell with off-diagonal permittivity
    /// (ε_xy or ε_yx): a mirror would change their sign, so the structure isn't symmetric
    /// about the wall.
    pub fn with_boundaries(mut self, boundaries: Boundaries) -> Result<CrossSection> {
        let (nx, ny) = (self.x.len() - 1, self.y.len() - 1);
        let off_diagonal = |i: usize, j: usize| {
            let e = self.cells[i * ny + j];
            e.xy.norm() > 0.0 || e.yx.norm() > 0.0
        };
        let edges = [
            ("west", boundaries.west, (0..ny).any(|j| off_diagonal(0, j))),
            (
                "east",
                boundaries.east,
                (0..ny).any(|j| off_diagonal(nx - 1, j)),
            ),
            (
                "south",
                boundaries.south,
                (0..nx).any(|i| off_diagonal(i, 0)),
            ),
            (
                "north",
                boundaries.north,
                (0..nx).any(|i| off_diagonal(i, ny - 1)),
            ),
        ];
        for (name, boundary, off_diagonal) in edges {
            if boundary != Boundary::Zero && off_diagonal {
                return Err(Error::invalid(
                    "cross-section",
                    format!(
                        "a wall on the {name} edge needs the cells beside it to have ε_xy = ε_yx = 0"
                    ),
                ));
            }
        }
        self.boundaries = boundaries;
        Ok(self)
    }

    /// What lies at the window's edges.
    pub fn boundaries(&self) -> Boundaries {
        self.boundaries
    }

    /// The same cross-section with perfectly matched layers inside its edges.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a thickness or strength that is negative or not finite,
    /// layers that would meet (thicker together than the window), or a layer on an edge with a
    /// mirror wall (a wall is a plane of symmetry, not an open boundary).
    pub fn with_pml(mut self, pml: Pml) -> Result<CrossSection> {
        let bad = |v: f64| !(v.is_finite() && v >= 0.0);
        if [pml.west, pml.east, pml.south, pml.north, pml.strength]
            .into_iter()
            .any(bad)
        {
            return Err(Error::invalid(
                "PML",
                "thicknesses and strength must be finite and not negative",
            ));
        }
        let span = |v: &[f64]| v[v.len() - 1] - v[0];
        if pml.west + pml.east >= span(&self.x) || pml.south + pml.north >= span(&self.y) {
            return Err(Error::invalid(
                "PML",
                "the layers are thicker than the window",
            ));
        }
        let b = self.boundaries;
        for (name, thickness, boundary) in [
            ("west", pml.west, b.west),
            ("east", pml.east, b.east),
            ("south", pml.south, b.south),
            ("north", pml.north, b.north),
        ] {
            if thickness > 0.0 && boundary != Boundary::Zero {
                return Err(Error::invalid(
                    "PML",
                    format!("the {name} edge has a mirror wall; a PML needs it open"),
                ));
            }
        }
        self.pml = pml;
        Ok(self)
    }

    /// The perfectly matched layers inside the window's edges.
    pub fn pml(&self) -> Pml {
        self.pml
    }

    /// The node coordinates along one axis, stretched into the complex plane inside the PML
    /// (Chew et al., Eq. 45): x̃ = x − iα u³/(3d²) at depth u into the low edge's layer of
    /// thickness d, and + at the high edge's.
    fn stretched(v: &[f64], low: f64, high: f64, strength: f64) -> Vec<c64> {
        let (start, end) = (v[0], v[v.len() - 1]);
        v.iter()
            .map(|&x| {
                let mut im = 0.0;
                if low > 0.0 && x < start + low {
                    let u = start + low - x;
                    im -= strength * u.powi(3) / (3.0 * low * low);
                }
                if high > 0.0 && x > end - high {
                    let u = x - (end - high);
                    im += strength * u.powi(3) / (3.0 * high * high);
                }
                c64::new(x, im)
            })
            .collect()
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

    /// A waveguide bent in the x–z plane around a centre at x = −`radius` (µm). `eps` gives the
    /// straight cross-section's permittivity at (x, y), x being the physical distance from the
    /// reference radius, positive towards the outside of the bend: a strip at |x| < w/2 is
    /// centred on it. The nodes are uniform in x, `nx` × `ny` cells from (x0, y0) to (x1, y1),
    /// so interfaces placed on nodes stay on nodes.
    ///
    /// M. Heiblum, J. H. Harris, IEEE J. Quantum Electron. 11, 75 (1975),
    /// [doi:10.1109/JQE.1975.1068563](https://doi.org/10.1109/JQE.1975.1068563), Eqs. (9)–(10):
    /// the map u = R ln(1 + x/R) turns the bend into a straight guide whose permittivity is
    /// multiplied by e^(2u/R) = (1 + x/R)². The cross-section's x coordinates are u: each node
    /// is mapped, so the grid is uniform in x and not in u, and each cell takes `eps` at its
    /// centre times e^(2u/R) at its centre in u. A mode's effective index is β/k with β the
    /// propagation constant along the arc at the reference radius, and its loss per unit
    /// length there follows from Im n_eff (see [`crate::mode::dispersion::bend_loss_db`]).
    ///
    /// The map is exact for the scalar wave equation (Heiblum and Harris's Eq. 3), so for the
    /// polarization with E normal to the bend plane (E along y in a slab uniform in y). For E in
    /// the bend plane the exact equivalent medium is anisotropic in ε and μ; scaling the
    /// isotropic ε, as here, adds a term whose first-order effect on a bound mode vanishes, so
    /// the error is of order 1/R². The outside of a bend radiates: give it a [`Pml`] on the
    /// east edge, its thickness in u.
    ///
    /// # Errors
    ///
    /// As [`CrossSection::new`], and [`Error::InvalidValue`] for a radius that isn't positive
    /// and finite, or a window reaching the centre of the bend (x0 ≤ −radius).
    pub fn bent(
        radius: f64,
        (x0, x1, nx): (f64, f64, usize),
        (y0, y1, ny): (f64, f64, usize),
        eps: impl Fn(f64, f64) -> Permittivity,
    ) -> Result<CrossSection> {
        if !(radius.is_finite() && radius > 0.0) {
            return Err(Error::invalid(
                "bend",
                format!("the radius must be positive, got {radius}"),
            ));
        }
        if x0 <= -radius {
            return Err(Error::invalid(
                "bend",
                format!(
                    "the window starts at x = {x0}, at or beyond the bend's centre (-{radius})"
                ),
            ));
        }
        let line = |a: f64, b: f64, n: usize| -> Vec<f64> {
            (0..=n).map(|i| a + (b - a) * i as f64 / n as f64).collect()
        };
        let (xs, y) = (line(x0, x1, nx), line(y0, y1, ny));
        let u: Vec<f64> = xs.iter().map(|&x| radius * (x / radius).ln_1p()).collect();
        let mut cells = Vec::with_capacity(nx * ny);
        for i in 0..nx {
            let x = 0.5 * (xs[i] + xs[i + 1]);
            let f = (u[i] + u[i + 1]) / radius;
            let f = f.exp();
            for j in 0..ny {
                let e = eps(x, 0.5 * (y[j] + y[j + 1]));
                cells.push(Permittivity {
                    xx: e.xx * f,
                    xy: e.xy * f,
                    yx: e.yx * f,
                    yy: e.yy * f,
                    zz: e.zz * f,
                });
            }
        }
        CrossSection::new(u, y, cells)
    }

    /// How many unknowns the cross-section's eigenproblem has: H_x and H_y at each node, less
    /// the components its walls hold at zero.
    pub fn unknowns(&self) -> usize {
        unknowns(self).len()
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
    pub(crate) fn cell(&self, i: isize, j: isize) -> Permittivity {
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

/// A mode search's `near`, an effective index to look about: finite when given (a NaN would
/// only surface as an eigensolver that doesn't converge).
pub(crate) fn check_near(near: Option<f64>) -> Result<()> {
    match near {
        Some(n) if !n.is_finite() => Err(Error::invalid(
            "mode search",
            format!("the effective index to look near must be finite, got {n}"),
        )),
        _ => Ok(()),
    }
}

/// The unknowns: H_x and H_y at every node (i, j), numbered i·(ny + 1) + j, except a component
/// that is odd across a wall at the wall's own nodes, where it is zero.
fn unknowns(cs: &CrossSection) -> Vec<(Component, usize)> {
    let (nxn, nyn) = (cs.x.len(), cs.y.len());
    let b = cs.boundaries;
    let odd = |boundary, normal, c| parity(boundary, normal, c) == Some(-1.0);
    let mut out = Vec::with_capacity(2 * nxn * nyn);
    for c in [Component::X, Component::Y] {
        for i in 0..nxn {
            for j in 0..nyn {
                let zero = (i == 0 && odd(b.west, Component::X, c))
                    || (i == nxn - 1 && odd(b.east, Component::X, c))
                    || (j == 0 && odd(b.south, Component::Y, c))
                    || (j == nyn - 1 && odd(b.north, Component::Y, c));
                if !zero {
                    out.push((c, i * nyn + j));
                }
            }
        }
    }
    out
}

/// The [`unknowns`] of a cross-section, and the unknown each stencil neighbour stands for.
pub(crate) struct Numbering {
    /// The unknowns in order: component and node.
    pub(crate) unknowns: Vec<(Component, usize)>,
    index: [Vec<Option<usize>>; 2],
    nxn: usize,
    nyn: usize,
    boundaries: Boundaries,
}

impl Numbering {
    pub(crate) fn new(cs: &CrossSection) -> Numbering {
        let (nxn, nyn) = (cs.x.len(), cs.y.len());
        let nodes = nxn * nyn;
        let unknowns = unknowns(cs);
        let mut index = [vec![None; nodes], vec![None; nodes]];
        for (k, &(c, node)) in unknowns.iter().enumerate() {
            index[c as usize][node] = Some(k);
        }
        Numbering {
            unknowns,
            index,
            nxn,
            nyn,
            boundaries: cs.boundaries,
        }
    }

    /// The unknown that a stencil's neighbour (i, j), of component c, stands for, with its sign:
    /// beyond a wall, its mirror image; beyond a zero boundary, or held at zero, none.
    pub(crate) fn neighbour(&self, c: Component, i: isize, j: isize) -> Option<(usize, f64)> {
        let b = self.boundaries;
        let mut sign = 1.0;
        let mut fold = |v: isize, last: isize, low: Boundary, high: Boundary, normal| {
            if v < 0 {
                sign *= parity(low, normal, c)?;
                Some(-v)
            } else if v > last {
                sign *= parity(high, normal, c)?;
                Some(2 * last - v)
            } else {
                Some(v)
            }
        };
        let i = fold(i, self.nxn as isize - 1, b.west, b.east, Component::X)?;
        let j = fold(j, self.nyn as isize - 1, b.south, b.north, Component::Y)?;
        self.index[c as usize][i as usize * self.nyn + j as usize].map(|k| (k, sign))
    }

    /// The row of component `c` at node (i, j), unless it is held at zero.
    pub(crate) fn row(&self, c: Component, i: usize, j: usize) -> Option<usize> {
        self.index[c as usize][i * self.nyn + j]
    }
}

/// The matrix of Eq. (8), as (row, column, value) entries over the [`unknowns`].
fn assemble(cs: &CrossSection, k2: f64) -> Vec<(usize, usize, c64)> {
    let (nxn, nyn) = (cs.x.len(), cs.y.len());
    let nodes = nxn * nyn;
    let numbering = Numbering::new(cs);
    let neighbour = |c, i, j| numbering.neighbour(c, i, j);
    let pml = cs.pml;
    let xs = CrossSection::stretched(&cs.x, pml.west, pml.east, pml.strength);
    let ys = CrossSection::stretched(&cs.y, pml.south, pml.north, pml.strength);
    let spacing = |v: &[c64], i: usize, ahead: bool| -> c64 {
        // at the window's edge the spacing continues: to the zero beyond it, or to the mirror
        // image of the first node inside a wall
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
            let (n, s) = (spacing(&ys, j, true), spacing(&ys, j, false));
            let (e, w) = (spacing(&xs, i, true), spacing(&xs, i, false));
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
            for (component, stencils) in [
                (
                    Component::X,
                    [(&axx, Component::X, false), (&axy, Component::Y, false)],
                ),
                (
                    Component::Y,
                    [(&ayx, Component::X, true), (&ayy, Component::Y, true)],
                ),
            ] {
                let Some(row) = numbering.row(component, i, j) else {
                    continue;
                };
                for (st, c, mirrored) in stencils {
                    for (di, dj, v) in around(st, mirrored) {
                        if v.re == 0.0 && v.im == 0.0 {
                            continue;
                        }
                        // folded neighbours can repeat a column: the matrix sums them
                        if let Some((q, sign)) = neighbour(c, ii + di, jj + dj) {
                            entries.push((row, q, v * sign));
                        }
                    }
                }
            }
        }
    }
    entries
}

/// Grid nodes from `a` to `b` (µm): spacing `h` inside the box `fine` (from, to), growing
/// outside it by `growth` per µm of distance from the box, up to `max`. The box's ends and the
/// points in `fixed` (interfaces) are nodes. Fine where the field changes fast (a waveguide's
/// core), coarse where it doesn't: keep `growth` small (a few hundredths), since the scheme loses
/// accuracy where neighbouring spacings differ much.
///
/// # Panics
///
/// Unless `a` and `b` are finite with `a` < `b`, `h` is positive and finite, `growth` is finite
/// and at least 0, and `max`, the box and `fixed` hold no NaN: the nodes would otherwise never
/// reach `b`.
pub fn graded_nodes(
    a: f64,
    b: f64,
    fine: (f64, f64),
    h: f64,
    growth: f64,
    max: f64,
    fixed: &[f64],
) -> Vec<f64> {
    assert!(
        a.is_finite() && b.is_finite() && a < b,
        "graded_nodes needs finite ends a < b, got {a} and {b}"
    );
    assert!(
        h.is_finite() && h > 0.0 && growth.is_finite() && growth >= 0.0,
        "graded_nodes needs a positive, finite spacing h and a finite growth of at least 0, got \
         h = {h} and growth = {growth}"
    );
    assert!(
        ![max, fine.0, fine.1]
            .iter()
            .chain(fixed)
            .any(|p| p.is_nan()),
        "graded_nodes needs a maximum, a box and fixed points that are numbers"
    );
    let step = |x: f64| -> f64 {
        let d = if x < fine.0 {
            fine.0 - x
        } else if x > fine.1 {
            x - fine.1
        } else {
            0.0
        };
        (h + growth * d).min(max.max(h))
    };
    let mut stops: Vec<f64> = [fine.0, fine.1]
        .iter()
        .chain(fixed)
        .copied()
        .filter(|&p| p > a && p < b)
        .collect();
    stops.push(b);
    stops.sort_by(f64::total_cmp);
    stops.dedup_by(|x, y| (*x - *y).abs() < 1e-12);
    let mut nodes = vec![a];
    for stop in stops {
        let start = nodes.last().copied().unwrap_or(a);
        let mut marks = vec![start];
        while marks.last().copied().unwrap_or(stop) < stop - 1e-12 {
            let y = marks.last().copied().unwrap_or(stop);
            marks.push((y + step(y)).min(stop));
        }
        // a sliver before the stop: share the last two steps evenly
        let n = marks.len();
        if n > 2 && stop - marks[n - 2] < 0.5 * step(marks[n - 2]) {
            marks[n - 2] = 0.5 * (marks[n - 3] + stop);
        }
        nodes.extend(marks.into_iter().skip(1));
    }
    nodes
}

/// Richardson extrapolation from results on three grids, each half the spacing of the last:
/// the convergence order p is fitted from the real parts, (v₀ − v₁)/(v₁ − v₂) = 2^p, and the
/// limit is v₂ − (v₁ − v₂)/(2^p − 1), for real and imaginary parts alike. Returns the limit and p.
/// Meaningful once the three are in the asymptotic range (the ratio steady).
pub fn richardson(values: [c64; 3]) -> (c64, f64) {
    let ratio = (values[0].re - values[1].re) / (values[1].re - values[2].re);
    let order = ratio.log2();
    (values[2] - (values[1] - values[2]) / (ratio - 1.0), order)
}

/// A full-vector mode of a cross-section.
#[derive(Clone, Debug, PartialEq)]
pub struct VectorMode {
    beta2: c64,
    k: f64,
    x: Vec<f64>,
    y: Vec<f64>,
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
        self.hx[i * self.y.len() + j]
    }

    /// H_y at node (i, j), on the same scale as [`VectorMode::hx`].
    pub fn hy(&self, i: usize, j: usize) -> c64 {
        self.hy[i * self.y.len() + j]
    }

    /// The share of the transverse magnetic field in H_y: near 1 for a TE-like mode (E mostly
    /// along x), near 0 for a TM-like one.
    pub fn te_fraction(&self) -> f64 {
        let x: f64 = self.hx.iter().map(|v| v.norm_sqr()).sum();
        let y: f64 = self.hy.iter().map(|v| v.norm_sqr()).sum();
        y / (x + y)
    }

    /// All six field components at the centres of `cs`'s cells (see [`crate::mode::fields`]):
    /// H_z from ∇·H = 0, E_z = (i/kε_zz)(∂_x H_y − ∂_y H_x) from Ampère's law, and the
    /// transverse E from Faraday's, E_x = (k/β) H_y + ∂_x E_z/(iβ) and
    /// E_y = −(k/β) H_x + ∂_y E_z/(iβ): the large transverse field straight from H, derivatives
    /// only in the small longitudinal correction, and E_z continuous across every interface.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless `cs` is the cross-section the mode was found on (the same
    /// grid).
    pub fn fields(&self, cs: &CrossSection) -> Result<crate::mode::fields::Fields> {
        if cs.x != self.x || cs.y != self.y {
            return Err(Error::invalid(
                "fields",
                "the mode was found on another grid",
            ));
        }
        let (nx, ny) = (self.x.len(), self.y.len());
        let (cx, cy) = (nx - 1, ny - 1);
        let beta = self.beta2.sqrt();
        let i = c64::new(0.0, 1.0);
        let at = |v: &[c64], a: usize, b: usize| v[a * ny + b];
        let xc: Vec<f64> = (0..cx).map(|a| 0.5 * (self.x[a] + self.x[a + 1])).collect();
        let yc: Vec<f64> = (0..cy).map(|b| 0.5 * (self.y[b] + self.y[b + 1])).collect();
        // per cell: H's average, ∂x and ∂y of H's transverse parts, and E_z
        let mut h = Vec::with_capacity(cx * cy);
        let mut ez = Vec::with_capacity(cx * cy);
        let mut area = Vec::with_capacity(cx * cy);
        for a in 0..cx {
            let dx = self.x[a + 1] - self.x[a];
            for b in 0..cy {
                let dy = self.y[b + 1] - self.y[b];
                let avg = |v: &[c64]| {
                    0.25 * (at(v, a, b) + at(v, a + 1, b) + at(v, a, b + 1) + at(v, a + 1, b + 1))
                };
                let ddx = |v: &[c64]| {
                    0.5 * ((at(v, a + 1, b) + at(v, a + 1, b + 1))
                        - (at(v, a, b) + at(v, a, b + 1)))
                        / dx
                };
                let ddy = |v: &[c64]| {
                    0.5 * ((at(v, a, b + 1) + at(v, a + 1, b + 1))
                        - (at(v, a, b) + at(v, a + 1, b)))
                        / dy
                };
                // ∇·H = 0: ∂x Hx + ∂y Hy + iβ Hz = 0
                let hz = i / beta * (ddx(&self.hx) + ddy(&self.hy));
                h.push([avg(&self.hx), avg(&self.hy), hz]);
                // Ampère, z: ∂x Hy − ∂y Hx = −ik ε_zz E_z
                ez.push(
                    i / (self.k * cs.cell(a as isize, b as isize).zz)
                        * (ddx(&self.hy) - ddy(&self.hx)),
                );
                area.push(dx * dy);
            }
        }
        // ∂x E_z, ∂y E_z at the cells' centres from their neighbours (one-sided at the edges)
        let d = |a: usize, b: usize, along_x: bool| -> c64 {
            let (len, centres, idx) = if along_x { (cx, &xc, a) } else { (cy, &yc, b) };
            if len < 2 {
                return c64::new(0.0, 0.0);
            }
            let (lo, hi) = (idx.saturating_sub(1), (idx + 1).min(len - 1));
            let (vlo, vhi) = if along_x {
                (ez[lo * cy + b], ez[hi * cy + b])
            } else {
                (ez[a * cy + lo], ez[a * cy + hi])
            };
            (vhi - vlo) / (centres[hi] - centres[lo])
        };
        let kb = self.k / beta;
        let mut e = Vec::with_capacity(cx * cy);
        for a in 0..cx {
            for b in 0..cy {
                let n = a * cy + b;
                let hc = h[n];
                // Faraday: iβ Ex − ∂x Ez = ik Hy, ∂y Ez − iβ Ey = ik Hx
                e.push([
                    kb * hc[1] + d(a, b, true) / (i * beta),
                    -kb * hc[0] + d(a, b, false) / (i * beta),
                    ez[n],
                ]);
            }
        }
        Ok(crate::mode::fields::Fields { xc, yc, area, e, h })
    }

    /// How alike two modes' transverse magnetic fields are, from 0 to 1:
    /// |∫ H_a* · H_b| / (‖H_a‖ ‖H_b‖), each node weighted by the area around it. `None` unless
    /// both modes are on the same grid. Recognizes a mode at a nearby wavelength or geometry.
    pub fn overlap(&self, other: &VectorMode) -> Option<f64> {
        if self.x != other.x || self.y != other.y {
            return None;
        }
        // the area around a node: half of each neighbouring spacing, along each axis
        let half = |v: &[f64], i: usize| {
            let a = if i > 0 { v[i] - v[i - 1] } else { 0.0 };
            let b = if i + 1 < v.len() {
                v[i + 1] - v[i]
            } else {
                0.0
            };
            0.5 * (a + b)
        };
        let (mut ab, mut aa, mut bb) = (c64::new(0.0, 0.0), 0.0, 0.0);
        for i in 0..self.x.len() {
            for j in 0..self.y.len() {
                let w = half(&self.x, i) * half(&self.y, j);
                let n = i * self.y.len() + j;
                ab += (self.hx[n].conj() * other.hx[n] + self.hy[n].conj() * other.hy[n]) * w;
                aa += (self.hx[n].norm_sqr() + self.hy[n].norm_sqr()) * w;
                bb += (other.hx[n].norm_sqr() + other.hy[n].norm_sqr()) * w;
            }
        }
        Some(ab.norm() / (aa * bb).sqrt())
    }
}

/// The `count` modes of `cs` at `wavelength` with effective indices nearest `near` (the
/// highest index in the cross-section when `None`: the fundamental modes), nearest first.
///
/// # Errors
///
/// [`Error::InvalidValue`] for a count of 0, a `near` that isn't finite, or if the eigenproblem
/// fails to converge.
pub fn modes(
    cs: &CrossSection,
    wavelength: Wavelength,
    count: usize,
    near: Option<f64>,
) -> Result<Vec<VectorMode>> {
    // (never asked to stop, it always has its modes or an error)
    Ok(modes_until(cs, wavelength, count, near, || false)?.unwrap_or_default())
}

/// As [`modes`], giving up when `stop` says so: `None` then. A solve for many modes on a fine
/// grid takes minutes, and a job's time limit or its stop must be able to end it: `stop` is
/// asked before each step of the eigensolver (one solve with the shifted matrix's factors
/// each). The factorization itself, before the first step, isn't interrupted.
///
/// # Errors
///
/// As [`modes`].
pub fn modes_until(
    cs: &CrossSection,
    wavelength: Wavelength,
    count: usize,
    near: Option<f64>,
    stop: impl Fn() -> bool,
) -> Result<Option<Vec<VectorMode>>> {
    if count == 0 {
        return Err(Error::invalid("mode count", "must be at least 1"));
    }
    check_near(near)?;
    let k = wavelength.wavenumber();
    let k2 = k * k;
    let n_max = near.unwrap_or_else(|| {
        cs.cells
            .iter()
            .map(|e| e.xx.re.max(e.yy.re).sqrt())
            .fold(1.0, f64::max)
    });
    let unknowns = unknowns(cs);
    let entries = assemble(cs, k2);
    let shift = c64::new(k2 * n_max * n_max, 0.0);
    let Some(pairs) =
        crate::eigen::nearest_until(unknowns.len(), &entries, shift, count, 1e-9, &stop)?
    else {
        return Ok(None);
    };
    Ok(Some(
        pairs
            .into_iter()
            .map(|p| VectorMode::from_unknowns(cs, k, p.value, &unknowns, &p.vector))
            .collect(),
    ))
}

impl VectorMode {
    /// How far the mode is from solving the eigenproblem of `cs` at its wavelength:
    /// ‖A h − β² h‖ / (|β²| ‖h‖), with A assembled afresh and h the mode's field over the
    /// unknowns. It is the measure [`modes`] accepts an eigenpair by (see `crate::eigen`), so
    /// for a mode it returned on `cs` it is below that tolerance, 1e-9.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if `cs` isn't on the mode's grid.
    pub fn residual(&self, cs: &CrossSection) -> Result<f64> {
        if cs.x != self.x || cs.y != self.y {
            return Err(Error::invalid(
                "cross-section",
                "must be the one the mode was solved on",
            ));
        }
        let unknowns = unknowns(cs);
        let h = self.to_unknowns(&unknowns);
        let mut ah = vec![c64::new(0.0, 0.0); h.len()];
        for (row, column, value) in assemble(cs, self.k * self.k) {
            ah[row] += value * h[column];
        }
        let off: f64 = ah
            .iter()
            .zip(&h)
            .map(|(a, x)| (a - self.beta2 * x).norm_sqr())
            .sum();
        let size: f64 = h.iter().map(|x| x.norm_sqr()).sum();
        Ok(off.sqrt() / (self.beta2.norm() * size.sqrt()))
    }

    /// The mode with eigenvalue β² whose field, over `unknowns`, is `vector`.
    pub(crate) fn from_unknowns(
        cs: &CrossSection,
        k: f64,
        beta2: c64,
        unknowns: &[(Component, usize)],
        vector: &[c64],
    ) -> VectorMode {
        let nodes = cs.x.len() * cs.y.len();
        // the components held at zero on walls are zero
        let (mut hx, mut hy) = (
            vec![c64::new(0.0, 0.0); nodes],
            vec![c64::new(0.0, 0.0); nodes],
        );
        for (&(c, node), &v) in unknowns.iter().zip(vector) {
            match c {
                Component::X => hx[node] = v,
                Component::Y => hy[node] = v,
            }
        }
        let peak = vector
            .iter()
            .copied()
            .max_by(|a, b| a.norm().total_cmp(&b.norm()))
            .unwrap_or(c64::new(1.0, 0.0));
        VectorMode {
            beta2,
            k,
            x: cs.x.clone(),
            y: cs.y.clone(),
            hx: hx.iter().map(|v| v / peak).collect(),
            hy: hy.iter().map(|v| v / peak).collect(),
        }
    }

    /// The field over `unknowns`, in their order.
    pub(crate) fn to_unknowns(&self, unknowns: &[(Component, usize)]) -> Vec<c64> {
        unknowns
            .iter()
            .map(|&(c, node)| match c {
                Component::X => self.hx[node],
                Component::Y => self.hy[node],
            })
            .collect()
    }
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

/// One of Hadley's four corner test problems on a uniform grid of `n` × `n` cells over the
/// 1 × 1 µm quarter domain, and its modal index at λ = 1.5 µm as published: G. R. Hadley,
/// J. Lightwave Technol. 20, 1219 (2002), doi:10.1109/JLT.2002.800371, Figs. 4–7.
///
/// 1 and 2 (Figs. 4–5) are boxes, ε = 2.25 or 8 for x, y < 0.5 µm and 1 elsewhere, with H_y
/// of zero derivative and H_x zero on all four edges; 3 and 4 (Figs. 6–7) are impinged
/// corners, ε = 1 for x, y > 0.5 µm and 2.25 or 8 elsewhere, with H_y of zero derivative on
/// the west and south edges and zero on the others, and H_x the opposite.
pub(crate) fn hadley_problem(problem: usize, n: usize) -> (CrossSection, f64) {
    use Boundary::{ElectricWall, MagneticWall};
    let (eps, boxed, expected) = match problem {
        1 => (2.25, true, 1.276_274_04),
        2 => (8.0, true, 2.656_796_92),
        3 => (2.25, false, 1.387_926_425),
        _ => (8.0, false, 2.761_465_320),
    };
    let cs = CrossSection::uniform((0.0, 1.0, n), (0.0, 1.0, n), |x, y| {
        let inside = if boxed {
            x < 0.5 && y < 0.5
        } else {
            !(x > 0.5 && y > 0.5)
        };
        Permittivity::isotropic(c64::new(if inside { eps } else { 1.0 }, 0.0))
    })
    .expect("a valid grid");
    // H_x zero on an edge normal to x is an electric wall; on an edge normal to y, a magnetic one
    let boundaries = if boxed {
        Boundaries {
            west: ElectricWall,
            east: ElectricWall,
            south: MagneticWall,
            north: MagneticWall,
        }
    } else {
        Boundaries {
            west: ElectricWall,
            east: MagneticWall,
            south: MagneticWall,
            north: ElectricWall,
        }
    };
    (
        cs.with_boundaries(boundaries).expect("isotropic cells"),
        expected,
    )
}

/// A slab lying flat (interfaces normal to y), uniform in x between two mirror walls, at grid
/// spacing `h` (µm): `layers` from the top down, each (index, thickness µm), the first and last
/// semi-infinite in reality and here `top` and `bottom` µm thick, a PML of `pml` µm and
/// strength `strength` below the bottom one, and a zero wall above the top one. TE (E along x)
/// behind electric walls, TM behind magnetic ones.
pub(crate) fn flat_slab(
    polarization: crate::mode::Polarization,
    layers: &[(f64, f64)],
    h: f64,
    pml: f64,
    strength: f64,
) -> CrossSection {
    let total: f64 = layers.iter().map(|&(_, t)| t).sum();
    let (y0, y1) = (-(total - layers[0].1) - pml, layers[0].1);
    let ny = ((y1 - y0) / h).round() as usize;
    let wall = match polarization {
        crate::mode::Polarization::Te => Boundary::ElectricWall,
        crate::mode::Polarization::Tm => Boundary::MagneticWall,
    };
    let index = |y: f64| {
        // the top layer spans (0, top]; each next one lies below the last
        let mut top = layers[0].1;
        for &(n, t) in layers {
            if y > top - t {
                return n;
            }
            top -= t;
        }
        layers[layers.len() - 1].0
    };
    CrossSection::uniform((0.0, 4.0 * h, 4), (y0, y1, ny), |_, y| {
        Permittivity::isotropic(c64::new(index(y).powi(2), 0.0))
    })
    .and_then(|cs| {
        cs.with_boundaries(Boundaries {
            west: wall,
            east: wall,
            ..Boundaries::default()
        })
    })
    .and_then(|cs| {
        cs.with_pml(Pml {
            south: pml,
            strength,
            ..Pml::default()
        })
    })
    .expect("a valid flat slab")
}

/// The leaky TE mode of 220 nm of silicon (3.473) in oxide (1.444) above a buried oxide
/// `box_um` thick on a silicon substrate, at 1.55 µm: from the full-vector solver at spacing `h`
/// with a 1 µm PML (α = 3) in the substrate, and exactly ([`crate::mode::multilayer`]).
pub(crate) fn soi_leakage(
    polarization: crate::mode::Polarization,
    box_um: f64,
    h: f64,
) -> (c64, c64) {
    use crate::mode::multilayer::Multilayer;
    use crate::units::Length;
    let (si, ox) = (3.473, 1.444);
    let w = Wavelength::from_um_unchecked(1.55);
    let stack = Multilayer::new(
        c64::new(ox, 0.0),
        &[
            (c64::new(si, 0.0), Length::nm(220.0)),
            (c64::new(ox, 0.0), Length::um(box_um)),
        ],
        c64::new(si, 0.0),
    )
    .expect("a valid stack");
    let guess = crate::mode::slab::Slab::new(ox, si, ox, Length::nm(220.0))
        .expect("a valid slab")
        .modes(polarization, w)[0]
        .effective_index();
    let exact = stack
        .mode_near(polarization, w, c64::new(guess, 1e-6))
        .expect("the leaky mode")
        .effective_index();
    let cs = flat_slab(
        polarization,
        &[(ox, 1.0), (si, 0.22), (ox, box_um), (si, 0.5)],
        h,
        1.0,
        3.0,
    );
    let found =
        modes(&cs, w, 1, Some(exact.re)).expect("the solver converges")[0].effective_index();
    (found, exact)
}

/// Chilwell and Hodgkinson's four-layer guide lying flat (cover 1.0, 1 µm, zero wall above;
/// films 1.66, 1.53, 1.60, 1.66 of 500 nm; substrate 1.50, 1 µm, over a 2 µm PML of α = 5) at
/// 632.8 nm and spacing `h`: its TE leaky waves nearest each of `near`.
pub(crate) fn chilwell_leaky(h: f64, near: &[c64]) -> Vec<c64> {
    let cs = flat_slab(
        crate::mode::Polarization::Te,
        &[
            (1.0, 1.0),
            (1.66, 0.5),
            (1.53, 0.5),
            (1.60, 0.5),
            (1.66, 0.5),
            (1.5, 1.0),
        ],
        h,
        2.0,
        5.0,
    );
    let w = Wavelength::from_um_unchecked(0.6328);
    near.iter()
        .map(|&target| {
            modes(&cs, w, 3, Some(target.re))
                .ok()
                .and_then(|m| {
                    m.iter()
                        .map(VectorMode::effective_index)
                        .min_by(|a, b| (a - target).norm().total_cmp(&(b - target).norm()))
                })
                .unwrap_or(c64::new(f64::NAN, f64::NAN))
        })
        .collect()
}

/// The book's strip as a 2D slab (its slab index, 2.845, 500 nm wide, in 1.444) bent at
/// `radius` µm, at 1.55 µm: from the full-vector solver on a bent cross-section at spacing `h`,
/// uniform along y between walls (E along y for TE, E in the bend plane for TM), with a PML on
/// the outside from x = 2.5 to 6 µm; and exactly ([`crate::mode::bend::SlabBend`]).
pub(crate) fn bent_slab(
    polarization: crate::mode::Polarization,
    radius: f64,
    h: f64,
) -> (c64, c64) {
    use crate::mode::Polarization;
    use crate::units::Length;
    let (core, clad, t) = (2.845, 1.444, 0.5);
    let w = Wavelength::from_um_unchecked(1.55);
    let (x0, x1) = ((-0.75f64).max(-0.9 * radius), 6.0);
    let pml = radius * (x1 / radius).ln_1p() - radius * (2.5 / radius).ln_1p();
    let wall = match polarization {
        Polarization::Te => Boundary::ElectricWall,
        Polarization::Tm => Boundary::MagneticWall,
    };
    let cs = CrossSection::bent(
        radius,
        (x0, x1, ((x1 - x0) / h).round() as usize),
        (0.0, 4.0 * h, 4),
        |x, _| {
            let n = if x.abs() < t / 2.0 { core } else { clad };
            Permittivity::isotropic(c64::new(n * n, 0.0))
        },
    )
    .and_then(|cs| {
        cs.with_boundaries(Boundaries {
            south: wall,
            north: wall,
            ..Boundaries::default()
        })
    })
    .and_then(|cs| {
        cs.with_pml(Pml {
            east: pml,
            strength: 3.0,
            ..Pml::default()
        })
    })
    .expect("a valid bent slab");
    let found = modes(&cs, w, 1, Some(core)).expect("the solver converges")[0].effective_index();
    let exact = crate::mode::bend::SlabBend::new(
        Length::um(radius),
        clad,
        &[(core, Length::um(t))],
        clad,
        Length::um(-t / 2.0),
    )
    .and_then(|b| b.mode_near(polarization, w, found))
    .expect("the exact bend");
    (found, exact)
}

/// The benchmark leaky photonic wire of P. Bienstman et al., Opt. Quantum Electron. 38, 731
/// (2006): 500 × 220 nm of silicon (3.5) on 1 µm of oxide (1.45) on a silicon substrate, air
/// above, at 1.55 µm. Its right half behind an electric wall, the core's box at spacing `h`, the
/// rest growing to 10 nm, a 1 µm PML in the substrate; the TE-like mode's effective index.
pub(crate) fn bienstman_wire(h: f64) -> c64 {
    let (si, ox) = (3.5, 1.45);
    let x = graded_nodes(0.0, 1.6, (0.0, 0.35), h, 0.0125, 0.01, &[0.25]);
    let y = graded_nodes(
        -2.6,
        1.5,
        (-0.1, 0.32),
        h,
        0.0125,
        0.01,
        &[0.0, 0.22, -1.0, -1.6],
    );
    let mut cells = Vec::with_capacity((x.len() - 1) * (y.len() - 1));
    for i in 0..x.len() - 1 {
        let xc = 0.5 * (x[i] + x[i + 1]);
        for j in 0..y.len() - 1 {
            let yc = 0.5 * (y[j] + y[j + 1]);
            let n: f64 = if yc > 0.22 {
                1.0
            } else if yc > 0.0 {
                if xc < 0.25 { si } else { 1.0 }
            } else if yc > -1.0 {
                ox
            } else {
                si
            };
            cells.push(Permittivity::isotropic(c64::new(n * n, 0.0)));
        }
    }
    let cs = CrossSection::new(x, y, cells)
        .and_then(|cs| {
            cs.with_boundaries(Boundaries {
                west: Boundary::ElectricWall,
                ..Boundaries::default()
            })
        })
        .and_then(|cs| {
            cs.with_pml(Pml {
                south: 1.0,
                strength: 3.0,
                ..Pml::default()
            })
        })
        .expect("a valid wire");
    modes(&cs, Wavelength::from_um_unchecked(1.55), 1, Some(2.41)).expect("the solver converges")[0]
        .effective_index()
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
    fn a_modes_residual_is_within_the_solvers_tolerance_and_tells_a_wrong_eigenvalue() {
        let cs = strip(0.05);
        let w = Wavelength::um(1.55).unwrap();
        let mut mode = modes(&cs, w, 1, None).unwrap().remove(0);
        let solved = mode.residual(&cs).unwrap();
        assert!(solved < 1e-9, "{solved:e}");
        // the same field with β² 1 % off: A h − 1.01 β² h = −0.01 β² h, over 1.01 |β²| ‖h‖
        mode.beta2 *= 1.01;
        let off = mode.residual(&cs).unwrap();
        assert!((off - 0.01 / 1.01).abs() < 1e-8, "{off}");
        // on another grid there is nothing to measure against
        assert!(mode.residual(&strip(0.1)).is_err());
    }

    #[test]
    fn a_solve_asked_to_stop_gives_up_at_its_next_step() {
        let cs = strip(0.05);
        let w = Wavelength::um(1.55).unwrap();
        // asked to stop from the third question on: no modes, and no more than that was asked
        let asked = std::cell::Cell::new(0);
        let stopped = modes_until(&cs, w, 2, None, || {
            asked.set(asked.get() + 1);
            asked.get() >= 3
        })
        .unwrap();
        assert!(stopped.is_none());
        assert_eq!(asked.get(), 3);
        // not asked to stop: the modes `modes` finds
        let whole = modes(&cs, w, 2, None).unwrap();
        let until = modes_until(&cs, w, 2, None, || false).unwrap().unwrap();
        assert_eq!(whole.len(), 2);
        for (a, b) in whole.iter().zip(&until) {
            assert_eq!(a.effective_index(), b.effective_index());
        }
        // and a count of 0 is still an error
        assert!(modes_until(&cs, w, 0, None, || false).is_err());
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
        // 500 x 220 nm silicon in oxide at 1550 nm (TE 2.447067, 2.445713, 2.444396, 2.443506
        // at 20, 10, 5, 2.5 nm): the convex corners' first-order convergence, not yet
        // asymptotic, as on Hadley's boxes (hadley_convergence)
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

    fn core(inside: bool) -> Permittivity {
        iso(if inside {
            3.473f64.powi(2)
        } else {
            1.444f64.powi(2)
        })
    }

    #[test]
    fn walls_give_the_exact_slab_in_either_orientation_at_second_order() {
        // the book's slab, uniform along one axis between two walls: a mode uniform along it
        // is exactly the slab's, with interfaces normal to x (Eqs. (21)–(35)) or to y (the
        // transposed Eq. (36)); the errors agree to 1e-12 (TE −3.90e-5 at 2.5 nm, TM +5.26e-6)
        let lam = Wavelength::um(1.55).unwrap();
        let slab =
            crate::mode::slab::Slab::new(1.444, 3.473, 1.444, crate::units::Length::nm(220.0))
                .unwrap();
        // TE (E along the interfaces): H tangential to the walls; TM: H normal to them
        for (pol, wall) in [
            (Polarization::Te, Boundary::ElectricWall),
            (Polarization::Tm, Boundary::MagneticWall),
        ] {
            let exact = slab.modes(pol, lam)[0].effective_index();
            let mut errors = [Vec::new(), Vec::new()];
            for h in [0.02f64, 0.01, 0.005] {
                let n = (4.02 / h).round() as usize;
                let along_x = CrossSection::uniform((-2.01, 2.01, n), (0.0, 0.08, 4), |x, _| {
                    core(x.abs() < 0.11)
                })
                .unwrap()
                .with_boundaries(Boundaries {
                    south: wall,
                    north: wall,
                    ..Default::default()
                })
                .unwrap();
                let along_y = CrossSection::uniform((0.0, 0.08, 4), (-2.01, 2.01, n), |_, y| {
                    core(y.abs() < 0.11)
                })
                .unwrap()
                .with_boundaries(Boundaries {
                    west: wall,
                    east: wall,
                    ..Default::default()
                })
                .unwrap();
                for (k, cs) in [along_x, along_y].iter().enumerate() {
                    errors[k].push(
                        modes(cs, lam, 1, Some(exact)).unwrap()[0]
                            .effective_index()
                            .re
                            - exact,
                    );
                }
            }
            for (a, b) in errors[0].iter().zip(&errors[1]) {
                assert!((a - b).abs() < 1e-12, "{pol:?}: {a} vs {b}");
            }
            for w in errors[0].windows(2) {
                let order = (w[0] / w[1]).abs().log2();
                assert!(
                    (order - 2.0).abs() < 0.1,
                    "{pol:?}: order {order}, {errors:?}"
                );
            }
        }
    }

    #[test]
    fn walls_fold_a_symmetric_waveguide_exactly() {
        // a strip symmetric about x = 0 and y = 0: on the full window, on the half x ≥ 0 and on
        // the quarter x, y ≥ 0, it is one discrete problem, and its modes agree to round-off
        use Boundary::{ElectricWall, MagneticWall};
        let lam = Wavelength::um(1.55).unwrap();
        let eps = |x: f64, y: f64| core(x.abs() < 0.24 && y.abs() < 0.1);
        let full = CrossSection::uniform((-1.0, 1.0, 100), (-0.7, 0.7, 70), eps).unwrap();
        let folded = |west, south| {
            let (y0, ny) = if south == Boundary::Zero {
                (-0.7, 70)
            } else {
                (0.0, 35)
            };
            CrossSection::uniform((0.0, 1.0, 50), (y0, 0.7, ny), eps)
                .unwrap()
                .with_boundaries(Boundaries {
                    west,
                    south,
                    ..Default::default()
                })
                .unwrap()
        };
        let both = modes(&full, lam, 2, None).unwrap();
        let (te, tm) = (both[0].effective_index(), both[1].effective_index());
        assert!(both[0].te_fraction() > 0.9 && both[1].te_fraction() < 0.1);
        // TE-like: H_x odd about x = 0, H_y odd about y = 0; TM-like the opposite
        for (cs, want) in [
            (folded(ElectricWall, Boundary::Zero), te),
            (folded(ElectricWall, MagneticWall), te),
            (folded(MagneticWall, Boundary::Zero), tm),
            (folded(MagneticWall, ElectricWall), tm),
        ] {
            let got = modes(&cs, lam, 1, None).unwrap()[0].effective_index();
            assert!((got - want).norm() < 1e-10, "{got} vs {want}");
        }
    }

    #[test]
    fn a_wall_beside_an_off_diagonal_cell_is_an_error() {
        let tilted = Permittivity {
            xy: c64::new(0.1, 0.0),
            yx: c64::new(0.1, 0.0),
            ..iso(4.0)
        };
        let cs = CrossSection::uniform((0.0, 1.0, 4), (0.0, 1.0, 4), |_, _| tilted).unwrap();
        let wall = Boundaries {
            north: Boundary::MagneticWall,
            ..Default::default()
        };
        assert!(cs.clone().with_boundaries(wall).is_err());
        assert!(cs.with_boundaries(Boundaries::default()).is_ok());
    }

    #[test]
    #[ignore = "a convergence study, run by hand (cargo test --release -- --ignored --nocapture)"]
    fn hadley_convergence() {
        // Hadley's four corner problems. Convex corners (the boxes, 1–2) converge at about
        // first order once asymptotic, the error changing sign on the way (problem 1: −1.3e-4,
        // +3.9e-5, +4.9e-5, +3.3e-5, +2.0e-5, +1.1e-5 at N = 20 … 640); concave corners
        // (impinged, 3–4) at 1.8 falling towards 1.4 (problem 4: −3.7e-4 … −1.4e-6)
        let lam = Wavelength::um(1.5).unwrap();
        for problem in 1..=4 {
            let mut last = f64::NAN;
            for n in [20, 40, 80, 160, 320, 640] {
                let (cs, expected) = hadley_problem(problem, n);
                let m = &modes(&cs, lam, 1, None).unwrap()[0];
                let err = m.effective_index().re - expected;
                println!(
                    "problem {problem} N {n:>3}: n_eff {:.9}  error {err:+.3e}  order {:.2}  TE {:.3}",
                    m.effective_index().re,
                    (last / err).abs().log2(),
                    m.te_fraction()
                );
                last = err;
            }
        }
    }

    #[test]
    fn the_pml_gives_a_leaky_slabs_exact_loss_at_second_order() {
        // 220 nm SOI on 0.5 µm of buried oxide, leaking into the substrate: the PML's loss
        // (Im n_eff) against the transfer-matrix solution, within 0.2 % at 5 nm and converging
        // at second order (TE: 0.68 %, 0.17 %, 0.04 % at 10, 5, 2.5 nm)
        for pol in [Polarization::Te, Polarization::Tm] {
            let (coarse, exact) = soi_leakage(pol, 0.5, 0.01);
            let (fine, _) = soi_leakage(pol, 0.5, 0.005);
            let err = |n: c64| (n.im / exact.im - 1.0).abs();
            assert!(err(fine) < 2e-3, "{pol:?}: {fine} vs {exact}");
            assert!(
                err(coarse) / err(fine) > 3.0,
                "{pol:?}: {coarse}, {fine} vs {exact}"
            );
            assert!(
                (fine.re - exact.re).abs() < 2e-4,
                "{pol:?}: {fine} vs {exact}"
            );
        }
    }

    #[test]
    fn a_pml_leaves_a_guided_mode_alone() {
        // the book's slab in oxide alone, guided: a PML below changes its index by round-off
        let lam = Wavelength::um(1.55).unwrap();
        let layers = [(1.444, 1.0), (3.473, 0.22), (1.444, 1.5)];
        let with = flat_slab(Polarization::Te, &layers, 0.01, 1.0, 3.0);
        let without = with.clone().with_pml(Pml::default()).unwrap();
        let a = modes(&with, lam, 1, None).unwrap()[0].effective_index();
        let b = modes(&without, lam, 1, None).unwrap()[0].effective_index();
        assert!((a - b).norm() < 1e-9 && a.im.abs() < 1e-9, "{a} vs {b}");
    }

    #[test]
    fn bad_pmls_are_errors() {
        let cs = CrossSection::uniform((0.0, 1.0, 10), (0.0, 1.0, 10), |_, _| iso(2.0)).unwrap();
        let pml = |west: f64, east: f64, strength: f64| Pml {
            west,
            east,
            strength,
            ..Pml::default()
        };
        assert!(cs.clone().with_pml(pml(-0.1, 0.0, 3.0)).is_err());
        assert!(cs.clone().with_pml(pml(0.2, 0.0, f64::NAN)).is_err());
        assert!(cs.clone().with_pml(pml(0.5, 0.5, 3.0)).is_err());
        assert!(cs.clone().with_pml(pml(0.2, 0.2, 3.0)).is_ok());
        let walled = cs
            .with_boundaries(Boundaries {
                west: Boundary::ElectricWall,
                ..Boundaries::default()
            })
            .unwrap();
        assert!(walled.with_pml(pml(0.2, 0.0, 3.0)).is_err());
    }

    #[test]
    fn a_bend_is_exact_for_e_normal_to_its_plane_at_second_order() {
        // E along y, normal to the bend plane: the conformal map is exact, and the solver
        // converges at second order onto the exact bent slab (R = 1 µm: −2.3e-4, −5.8e-5 at 10,
        // 5 nm), its loss within 0.1 %
        let (coarse, exact) = bent_slab(Polarization::Te, 1.0, 0.01);
        let (fine, _) = bent_slab(Polarization::Te, 1.0, 0.005);
        let order = ((coarse.re - exact.re) / (fine.re - exact.re)).log2();
        assert!(
            (order - 2.0).abs() < 0.1,
            "order {order}: {coarse}, {fine} vs {exact}"
        );
        assert!((fine.im / exact.im - 1.0).abs() < 2e-3, "{fine} vs {exact}");
    }

    #[test]
    fn a_bends_error_for_e_in_its_plane_falls_with_the_radius() {
        // E in the bend plane: scaling the isotropic ε is an approximation; its error, measured
        // against the exact bend, is 1.2e-3 at R = 1 µm and 1.3e-4 at 3 µm
        let error = |r: f64| {
            let (found, exact) = bent_slab(Polarization::Tm, r, 0.005);
            (found.re - exact.re).abs()
        };
        let (e1, e3) = (error(1.0), error(3.0));
        assert!(e1 < 2e-3 && e3 < 2e-4 && e3 < e1 / 5.0, "{e1}, {e3}");
    }

    #[test]
    fn graded_nodes_keep_their_box_and_interfaces() {
        let n = graded_nodes(-2.0, 3.0, (0.0, 0.5), 0.01, 0.05, 0.1, &[-1.0, 2.2]);
        assert_eq!((n[0], n[n.len() - 1]), (-2.0, 3.0));
        for p in [0.0, 0.5, -1.0, 2.2] {
            assert!(n.iter().any(|&x| (x - p).abs() < 1e-12), "{p} isn't a node");
        }
        assert!(
            n.windows(2)
                .all(|w| w[1] > w[0] && w[1] - w[0] <= 0.1 + 1e-12)
        );
        let inside: Vec<f64> = n
            .windows(2)
            .filter(|w| w[0] >= 0.0 && w[1] <= 0.5)
            .map(|w| w[1] - w[0])
            .collect();
        assert!(
            inside.iter().all(|&d| (d - 0.01).abs() < 1e-9),
            "{inside:?}"
        );
    }

    #[test]
    fn richardson_recovers_a_power_law() {
        // v(h) = 2 + 3 h^0.7 at h = 1, 1/2, 1/4: the limit 2, the order 0.7
        let v = |h: f64| c64::new(2.0 + 3.0 * h.powf(0.7), 1.0 + h.powf(0.7));
        let (limit, order) = richardson([v(1.0), v(0.5), v(0.25)]);
        assert!(
            (order - 0.7).abs() < 1e-12 && (limit - c64::new(2.0, 1.0)).norm() < 1e-12,
            "{limit}, {order}"
        );
    }

    #[test]
    fn bad_bends_are_errors() {
        let eps = |_: f64, _: f64| iso(2.0);
        assert!(CrossSection::bent(0.0, (-0.5, 0.5, 10), (0.0, 0.1, 2), eps).is_err());
        assert!(CrossSection::bent(1.0, (-1.0, 0.5, 10), (0.0, 0.1, 2), eps).is_err());
        assert!(CrossSection::bent(1.0, (-0.9, 0.5, 10), (0.0, 0.1, 2), eps).is_ok());
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

    #[test]
    fn a_search_near_a_nan_is_an_error_that_says_so() {
        // it used to reach the eigensolver and come back as "NoConvergence"
        let w = Wavelength::um(1.55).unwrap();
        let e = modes(&strip(0.05), w, 1, Some(f64::NAN))
            .unwrap_err()
            .to_string();
        assert!(e.contains("must be finite"), "{e}");
        assert!(modes(&strip(0.05), w, 1, Some(f64::INFINITY)).is_err());
    }

    #[test]
    #[should_panic(expected = "graded_nodes needs a positive, finite spacing h")]
    fn graded_nodes_with_no_spacing_panic_instead_of_filling_the_memory() {
        // h = 0 never reached b: it looped, allocating, until the memory ran out
        graded_nodes(-1.0, 1.0, (-0.2, 0.2), 0.0, 0.05, 0.1, &[]);
    }

    #[test]
    #[should_panic(expected = "graded_nodes needs finite ends a < b")]
    fn graded_nodes_to_infinity_panic() {
        graded_nodes(-1.0, f64::INFINITY, (-0.2, 0.2), 0.01, 0.05, 0.1, &[]);
    }
}
