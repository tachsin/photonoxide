//! Subpixel smoothing: the permittivity each value of E sees, averaged over a cell about it so
//! that an interface between two grid points is where the structure puts it.
//!
//! - **Isotropic media** (A. Farjadpour et al., Opt. Lett. 31, 2972 (2006),
//!   doi:10.1364/OL.31.002972, Eq. 1): ε̃⁻¹ = P⟨ε⁻¹⟩ + (1 − P)⟨ε⟩⁻¹, P = nnᵀ the projection on
//!   the interface's normal n, ⟨·⟩ the mean over the cell: the mean ε for the field along the
//!   interface, the harmonic mean across it. The perturbation the smoothing makes then has no
//!   first-order effect, and an interface anywhere in a cell keeps the scheme second order.
//! - **Anisotropic media** (C. Kottke, A. Farjadpour, S. G. Johnson, Phys. Rev. E 77, 036611
//!   (2008), doi:10.1103/PhysRevE.77.036611, Eqs. 4, 22 and 23): in the interface's frame (the
//!   first axis along n), ε̃ = τ⁻¹(⟨τ(ε)⟩), with τ(ε) the matrix of −1/ε₁₁, ε₁ⱼ/ε₁₁, εᵢ₁/ε₁₁ and
//!   εᵢⱼ − εᵢ₁ε₁ⱼ/ε₁₁, which maps the continuous (D₁, E₂, E₃) linearly; for isotropic media it is
//!   Eq. 1 again.
//! - **On Yee's grid** (A. F. Oskooi, C. Kottke, S. G. Johnson, Opt. Lett. 34, 2778 (2009),
//!   doi:10.1364/OL.34.002778, Fig. 1): each value of E keeps its own diagonal entry of ε̃⁻¹,
//!   averaged over the cell centred on it. The off-diagonal entries are averaged over the cells
//!   centred on the nodes and kept there ([`Coupling::Nodes`]): E_x at (i + ½, j, k) takes D_y
//!   from the two nodes beside it, at each the mean of D_y on either side times (ε̃⁻¹)_xy there,
//!   and the mean of the two nodes', G. R. Werner and J. R. Cary's scheme (J. Comput. Phys. 226,
//!   1085 (2007), doi:10.1016/j.jcp.2007.05.008, their Eq. 39), whose ε⁻¹ on the grid is symmetric, so that the leapfrog's energy
//!   ½ Σ E·D + ½ Σ H̃·H̃ is conserved as with a scalar ε. Or they are averaged over each value's
//!   own cell ([`Coupling::Points`]), times the mean of the four D_y around E_x, as Farjadpour et
//!   al. have it: second order at oblique interfaces, where the nodes' are first order, but not
//!   symmetric, and an error grows from round-off. Or each node's eight triplets of values take
//!   one tensor each ([`Coupling::Triplets`], G. R. Werner, C. A. Bauer, J. R. Cary, J. Comput.
//!   Phys. 255, 436 (2013), doi:10.1016/j.jcp.2013.08.009), C. A. Bauer, G. R. Werner and J. R.
//!   Cary's (J. Comput. Phys. 230, 2060 (2011), doi:10.1016/j.jcp.2010.12.005) made symmetric:
//!   positive definite at any contrast, where the nodes' isn't, and first order at oblique
//!   interfaces with half their error. A medium whose tensor couples E's components steps D, and
//!   E follows from it after each step.
//!
//! The cell is the grid's cell about each point, times a diameter (Farjadpour et al.'s s). Where
//! one body's surface crosses it, the surface is taken as the plane through the nearest point,
//! normal to the body's gradient there, and the share of the cell inside it is the exact volume
//! the plane cuts off: what ⟨·⟩ needs, with the normal. A curved surface's departure from that
//! plane, and an error of order d² in its distance d, are second order. Where two surfaces cross
//! a cell, it is sampled 8 times along each axis, in the frame of the uppermost surface.

use faer::Side;
use faer::sparse::{SparseColMat, Triplet};
use rayon::prelude::*;

use super::{Boundaries, Simulation, invalid};
use crate::Result;
use crate::fdfd::{Axis, Grid3d};
use crate::geometry::{Point, Shape};

type Matrix = [[f64; 3]; 3];

/// A relative permittivity: a real, symmetric, positive-definite 3 × 3 tensor in the grid's axes
/// (x, y, z).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Permittivity(Matrix);

impl Permittivity {
    /// The tensor `matrix`, its rows and columns along x, y and z.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] unless every entry is finite, the matrix is symmetric (to
    /// 1e-12 of its largest entry; it is made exactly so) and positive definite.
    pub fn new(matrix: [[f64; 3]; 3]) -> Result<Permittivity> {
        let scale = matrix.iter().flatten().fold(0.0f64, |m, v| m.max(v.abs()));
        let finite = matrix.iter().flatten().all(|v| v.is_finite());
        let symmetric =
            (0..3).all(|i| (0..3).all(|j| (matrix[i][j] - matrix[j][i]).abs() <= 1e-12 * scale));
        let m = symmetrized(matrix);
        // Sylvester's criterion: the leading principal minors are positive
        let minor = m[0][0] * m[1][1] - m[0][1] * m[1][0];
        if !(finite && symmetric && m[0][0] > 0.0 && minor > 0.0 && determinant(&m) > 0.0) {
            return Err(invalid(format!(
                "a permittivity must be finite, symmetric and positive definite, got {matrix:?}"
            )));
        }
        Ok(Permittivity(m))
    }

    /// An isotropic medium, ε times the identity.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] unless ε is finite and positive.
    pub fn isotropic(eps: f64) -> Result<Permittivity> {
        Permittivity::diagonal([eps; 3])
    }

    /// A tensor whose principal axes are the grid's: diag(ε_x, ε_y, ε_z).
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] unless every value is finite and positive.
    pub fn diagonal(values: [f64; 3]) -> Result<Permittivity> {
        let mut m = [[0.0; 3]; 3];
        for (i, v) in values.into_iter().enumerate() {
            m[i][i] = v;
        }
        Permittivity::new(m)
    }

    /// A tensor of principal values `values` along the principal axes `axes` (orthonormal, one
    /// per row): Σ εᵢ aᵢaᵢᵀ.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] unless the values are finite and positive and the axes
    /// orthonormal to 1e-9.
    pub fn principal(values: [f64; 3], axes: [[f64; 3]; 3]) -> Result<Permittivity> {
        if !orthonormal(&axes) {
            return Err(invalid(format!(
                "the principal axes must be orthonormal, got {axes:?}"
            )));
        }
        let m = std::array::from_fn(|i| {
            std::array::from_fn(|j| (0..3).map(|k| values[k] * axes[k][i] * axes[k][j]).sum())
        });
        if !values.iter().all(|v| v.is_finite() && *v > 0.0) {
            return Err(invalid(format!(
                "principal permittivities must be finite and positive, got {values:?}"
            )));
        }
        Permittivity::new(m)
    }

    /// A uniaxial crystal: `ordinary` across its optic axis `axis` (any length but zero),
    /// `extraordinary` along it, n_o² and n_e² for a crystal of indices n_o and n_e.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] unless both are finite and positive and the axis finite
    /// and not zero.
    pub fn uniaxial(ordinary: f64, extraordinary: f64, axis: [f64; 3]) -> Result<Permittivity> {
        let length = norm(axis);
        if !(length.is_finite() && length > 0.0) {
            return Err(invalid(format!("an optic axis can't be {axis:?}")));
        }
        let c = axis.map(|v| v / length);
        let m = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                let identity = if i == j { ordinary } else { 0.0 };
                identity + (extraordinary - ordinary) * c[i] * c[j]
            })
        });
        if !(ordinary.is_finite() && ordinary > 0.0) {
            return Err(invalid(format!(
                "the ordinary permittivity must be positive, got {ordinary}"
            )));
        }
        Permittivity::new(m)
    }

    /// The tensor, its rows and columns along x, y and z.
    pub fn matrix(&self) -> [[f64; 3]; 3] {
        self.0
    }
}

/// A region of space filled with one medium. Bodies are made by their functions, which check
/// them.
#[derive(Clone, Debug, PartialEq)]
pub struct Body(Kind);

#[derive(Clone, Debug, PartialEq)]
enum Kind {
    HalfSpace {
        point: [f64; 3],
        normal: [f64; 3],
    },
    Ellipsoid {
        center: [f64; 3],
        /// The reciprocal semi-axes, 0 for an infinite one.
        inverse: [f64; 3],
        axes: [[f64; 3]; 3],
    },
    Extruded(Shape),
}

impl Body {
    /// The half-space on the side of the plane through `point` (µm) that `normal` (any length
    /// but zero) points away from.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] unless everything is finite and the normal isn't zero.
    pub fn half_space(point: [f64; 3], normal: [f64; 3]) -> Result<Body> {
        let length = norm(normal);
        if !(point.iter().all(|v| v.is_finite()) && length.is_finite() && length > 0.0) {
            return Err(invalid(format!(
                "a half-space needs a finite point and normal, got {point:?} and {normal:?}"
            )));
        }
        Ok(Body(Kind::HalfSpace {
            point,
            normal: normal.map(|v| v / length),
        }))
    }

    /// An ellipsoid about `center` (µm) with semi-axes `semi_axes` (µm) along the orthonormal
    /// directions `axes` (one per row). A semi-axis may be infinite: one makes an elliptic
    /// cylinder, two a slab.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] unless the centre is finite, the semi-axes positive (at
    /// most two infinite) and the axes orthonormal to 1e-9.
    pub fn ellipsoid(center: [f64; 3], semi_axes: [f64; 3], axes: [[f64; 3]; 3]) -> Result<Body> {
        let finite = semi_axes.iter().filter(|s| s.is_finite()).count();
        if !(center.iter().all(|v| v.is_finite())
            && semi_axes.iter().all(|s| *s > 0.0)
            && finite >= 1
            && orthonormal(&axes))
        {
            return Err(invalid(format!(
                "an ellipsoid needs a finite centre, positive semi-axes (one at least finite) and \
                 orthonormal axes, got {center:?}, {semi_axes:?} and {axes:?}"
            )));
        }
        Ok(Body(Kind::Ellipsoid {
            center,
            inverse: semi_axes.map(|s| 1.0 / s),
            axes,
        }))
    }

    /// An elliptic cylinder along z: the ellipse about `center` (x, y, µm) with semi-axes
    /// `semi_axes` (µm), the first at `angle` (radians) from x, counterclockwise.
    ///
    /// # Errors
    ///
    /// As [`Body::ellipsoid`].
    pub fn ellipse(center: [f64; 2], semi_axes: [f64; 2], angle: f64) -> Result<Body> {
        let (s, c) = angle.sin_cos();
        Body::ellipsoid(
            [center[0], center[1], 0.0],
            [semi_axes[0], semi_axes[1], f64::INFINITY],
            [[c, s, 0.0], [-s, c, 0.0], [0.0, 0.0, 1.0]],
        )
    }

    /// A planar shape (as [`crate::geometry`] draws it, x and y in µm) extended without end
    /// along z.
    pub fn extruded(shape: Shape) -> Body {
        Body(Kind::Extruded(shape))
    }

    /// The signed distance from `p` to the surface, negative inside (exact for a half-space, a
    /// sphere and the planar shapes, to second order in itself for an ellipsoid), and the unit
    /// normal pointing out of the body at the nearest point of the surface.
    fn surface(&self, p: [f64; 3]) -> (f64, [f64; 3]) {
        match &self.0 {
            Kind::HalfSpace { point, normal } => (dot(*normal, sub(p, *point)), *normal),
            Kind::Ellipsoid {
                center,
                inverse,
                axes,
            } => {
                // u in the ellipsoid's own units, |u| = 1 on its surface; f = |u| − 1 has the
                // gradient g/|u|, g = Σ uᵢ aᵢ/sᵢ, and the distance is f/|∇f|: exact for a
                // sphere, second order in itself otherwise
                let q = sub(p, *center);
                let u: [f64; 3] = std::array::from_fn(|i| dot(q, axes[i]) * inverse[i]);
                let rho = norm(u);
                let g: [f64; 3] =
                    std::array::from_fn(|k| (0..3).map(|i| u[i] * inverse[i] * axes[i][k]).sum());
                let length = norm(g);
                if rho == 0.0 || length == 0.0 {
                    // the centre: deep inside
                    let smallest = inverse.iter().fold(0.0f64, |m, v| m.max(*v));
                    return (-1.0 / smallest, axes[0]);
                }
                ((rho - 1.0) * rho / length, g.map(|v| v / length))
            }
            Kind::Extruded(shape) => {
                let (d, n) = planar(shape, p[0], p[1]);
                (d, [n[0], n[1], 0.0])
            }
        }
    }
}

/// A planar shape's signed distance from (x, y), negative inside, and its outward normal at the
/// nearest point.
fn planar(shape: &Shape, x: f64, y: f64) -> (f64, [f64; 2]) {
    let radial = |center: &Point, x: f64, y: f64| {
        let (dx, dy) = (x - center.x.to_um(), y - center.y.to_um());
        let r = dx.hypot(dy);
        let n = if r > 0.0 {
            [dx / r, dy / r]
        } else {
            [1.0, 0.0]
        };
        (r, n)
    };
    match shape {
        Shape::Rect {
            center,
            width,
            height,
        } => {
            let (dx, dy) = (x - center.x.to_um(), y - center.y.to_um());
            let q = [
                dx.abs() - width.to_um() / 2.0,
                dy.abs() - height.to_um() / 2.0,
            ];
            let sign = [dx.signum(), dy.signum()];
            if q[0] > 0.0 || q[1] > 0.0 {
                let v = [q[0].max(0.0), q[1].max(0.0)];
                let d = v[0].hypot(v[1]);
                (d, [sign[0] * v[0] / d, sign[1] * v[1] / d])
            } else if q[0] > q[1] {
                (q[0], [sign[0], 0.0])
            } else {
                (q[1], [0.0, sign[1]])
            }
        }
        Shape::Circle { center, radius } => {
            let (r, n) = radial(center, x, y);
            (r - radius.to_um(), n)
        }
        Shape::Ring {
            center,
            inner,
            outer,
        } => {
            let (r, n) = radial(center, x, y);
            let (out, inn) = (r - outer.to_um(), inner.to_um() - r);
            if out >= inn {
                (out, n)
            } else {
                (inn, [-n[0], -n[1]])
            }
        }
        Shape::Polygon(polygon) => {
            let v = polygon.vertices();
            let mut best = (f64::INFINITY, [0.0, 0.0], [1.0, 0.0]);
            for i in 0..v.len() {
                let (a, b) = (v[i], v[(i + 1) % v.len()]);
                let (ax, ay, bx, by) = (a.x.to_um(), a.y.to_um(), b.x.to_um(), b.y.to_um());
                let (ex, ey) = (bx - ax, by - ay);
                let t = (((x - ax) * ex + (y - ay) * ey) / (ex * ex + ey * ey)).clamp(0.0, 1.0);
                let nearest = [ax + t * ex, ay + t * ey];
                let d = (x - nearest[0]).hypot(y - nearest[1]);
                if d < best.0 {
                    // counterclockwise: the edge's outward normal is (e_y, −e_x)
                    let l = ex.hypot(ey);
                    best = (d, nearest, [ey / l, -ex / l]);
                }
            }
            let (d, nearest, edge) = best;
            let inside = polygon.contains(Point::um(x, y));
            let n = if d > 0.0 {
                let away = [(x - nearest[0]) / d, (y - nearest[1]) / d];
                if inside { [-away[0], -away[1]] } else { away }
            } else {
                edge
            };
            (if inside { -d } else { d }, n)
        }
    }
}

/// A structure: a background medium and bodies over it, each over the ones before it.
#[derive(Clone, Debug, PartialEq)]
pub struct Structure {
    background: Permittivity,
    bodies: Vec<(Body, Permittivity)>,
}

/// How each value of E's permittivity is taken from a [`Structure`]: the average over a cell
/// about it, and where the tensor's off-diagonal entries are kept.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Smoothing {
    /// The average.
    pub average: Average,
    /// The cell's size in grid cells, Farjadpour et al.'s s: 1, or 2 as their Fig. 1 also has
    /// it. Unused by [`Average::Sampled`].
    pub diameter: f64,
    /// Where the off-diagonal entries of ε̃⁻¹ are kept, when there are any.
    pub coupling: Coupling,
}

impl Default for Smoothing {
    /// Subpixel smoothing over each value's own cell, the off-diagonal entries at the nodes.
    fn default() -> Smoothing {
        Smoothing::with(Average::Subpixel)
    }
}

impl Smoothing {
    /// `average` over each value's own cell, the off-diagonal entries at the nodes.
    pub fn with(average: Average) -> Smoothing {
        Smoothing {
            average,
            diameter: 1.0,
            coupling: Coupling::Nodes,
        }
    }
}

/// An average over a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Average {
    /// None: the permittivity at the value's own point. An interface between grid points moves
    /// to one of them, and the scheme is first order.
    Sampled,
    /// ⟨ε⟩ for every component (S. Dey, R. Mittra, IEEE Trans. Microw. Theory Tech. 47, 1737
    /// (1999)): wrong for the field across an interface, first order.
    Mean,
    /// ⟨ε⁻¹⟩⁻¹, the harmonic mean: wrong for the field along an interface, first order.
    InverseMean,
    /// Kottke et al.'s τ⁻¹(⟨τ⟩) in the interface's frame: ⟨ε⟩ along an interface between
    /// isotropic media and ⟨ε⁻¹⟩⁻¹ across it.
    Subpixel,
}

/// Where ε̃⁻¹'s off-diagonal entries are kept, and how E_x takes D_y and D_z.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coupling {
    /// At the nodes, Werner and Cary's (Oskooi et al.'s Fig. 1): E_x takes the mean of D_y on
    /// either side of each node beside it times (ε̃⁻¹)_xy there, and the mean of the two.
    /// Symmetric, so the leapfrog's energy is conserved and the scheme stable, while ε̃⁻¹ on
    /// the grid is positive definite, which a high contrast across an interface oblique to the
    /// grid breaks ([`Simulation::smoothed`] refuses it). Each row of ε̃⁻¹ mixes cells about
    /// different points: at an interface oblique to the grid, it is first
    /// order (docs/methods/fdtd.md, "Subpixel smoothing").
    Nodes,
    /// At each value of E, from its own cell, Farjadpour et al.'s: E_x takes (ε̃⁻¹)_xy at its
    /// own point times the mean of the four D_y around it. Second order at oblique interfaces,
    /// but not symmetric: an error grows exponentially from round-off, slowly in isotropic
    /// media and fast in anisotropic ones (docs/methods/fdtd.md), so for short runs only.
    Points,
    /// From each node's eight triplets, G. R. Werner, C. A. Bauer and J. R. Cary's scheme (J.
    /// Comput. Phys. 255, 436 (2013), doi:10.1016/j.jcp.2013.08.009): the values of E_x, E_y
    /// and E_z on the three edges from a node, one each way along each axis, see one 3 × 3
    /// tensor, and ε̃⁻¹ on the grid is the mean of the eight block-diagonal matrices so made,
    /// its diagonal entries too. Each tensor is C. A. Bauer, G. R. Werner and J. R. Cary's,
    /// exact for constant fields at a plane interface (J. Comput. Phys. 230, 2060 (2011),
    /// doi:10.1016/j.jcp.2010.12.005), made symmetric; one not positive definite, or a node
    /// two surfaces pass near, takes Kottke et al.'s average over the node's cell instead.
    /// ε̃⁻¹ is then symmetric and positive definite at any contrast, so the scheme is stable
    /// with no check. Like the nodes', first order at oblique interfaces, with a smaller error
    /// (docs/methods/fdtd.md). It takes its own average: [`Smoothing::average`] must be
    /// [`Average::Subpixel`] and the diameter 1.
    Triplets,
}

/// The samples per axis of a cell two surfaces cross.
const SAMPLES: usize = 8;

impl Structure {
    /// The background medium alone.
    pub fn new(background: Permittivity) -> Structure {
        Structure {
            background,
            bodies: Vec::new(),
        }
    }

    /// The structure with `body` of `eps` over everything before it.
    pub fn with(mut self, body: Body, eps: Permittivity) -> Structure {
        self.bodies.push((body, eps));
        self
    }

    /// What the box of `size` (µm) about `centre` holds: one medium, one body's surface (as the
    /// plane through its nearest point, as [`Structure::inverse`] takes it), or more.
    fn local(&self, centre: [f64; 3], size: [f64; 3]) -> Local {
        let mut crossing: Option<(f64, [f64; 3], Permittivity)> = None;
        let mut beneath = self.background;
        for (body, eps) in self.bodies.iter().rev() {
            let (d, n) = body.surface(centre);
            let reach = 0.5 * (0..3).map(|i| n[i].abs() * size[i]).sum::<f64>();
            if d <= -reach {
                beneath = *eps;
                break;
            }
            if d < reach {
                if crossing.is_some() {
                    return Local::Many;
                }
                crossing = Some((d, n, *eps));
            }
        }
        match crossing {
            None => Local::Uniform(beneath),
            Some((d, n, inside)) => Local::Plane {
                d,
                n,
                inside,
                beneath,
            },
        }
    }

    /// The permittivity at `p` (µm): the last body's that contains it, or the background.
    pub fn at(&self, p: [f64; 3]) -> Permittivity {
        self.bodies
            .iter()
            .rev()
            .find(|(body, _)| body.surface(p).0 < 0.0)
            .map_or(self.background, |&(_, eps)| eps)
    }

    /// ε̃⁻¹ over the cell of `size` (µm) about `centre`, by `average`.
    fn inverse(&self, centre: [f64; 3], size: [f64; 3], average: Average) -> Matrix {
        if average == Average::Sampled {
            return inverse(&self.at(centre).0);
        }
        // the bodies whose surface crosses the cell, from the top, down to one that covers it
        let mut crossing: Vec<(f64, [f64; 3], Permittivity)> = Vec::new();
        let mut beneath = self.background;
        for (body, eps) in self.bodies.iter().rev() {
            let (d, n) = body.surface(centre);
            let reach = 0.5 * (0..3).map(|i| n[i].abs() * size[i]).sum::<f64>();
            if d <= -reach {
                beneath = *eps;
                break;
            }
            if d < reach {
                crossing.push((d, n, *eps));
            }
        }
        let Some(&(d, n, inside)) = crossing.first() else {
            return inverse(&beneath.0);
        };
        let frame = frame(n);
        // each medium with its share of the cell
        let shares: Vec<(f64, Permittivity)> = if crossing.len() == 1 {
            let f = fill(n, d, size);
            vec![(f, inside), (1.0 - f, beneath)]
        } else {
            let offset = |s: usize| (s as f64 + 0.5) / SAMPLES as f64 - 0.5;
            let weight = 1.0 / (SAMPLES * SAMPLES * SAMPLES) as f64;
            let mut shares = Vec::with_capacity(SAMPLES * SAMPLES * SAMPLES);
            for a in 0..SAMPLES {
                for b in 0..SAMPLES {
                    for c in 0..SAMPLES {
                        let p = [
                            centre[0] + offset(a) * size[0],
                            centre[1] + offset(b) * size[1],
                            centre[2] + offset(c) * size[2],
                        ];
                        shares.push((weight, self.at(p)));
                    }
                }
            }
            shares
        };
        match average {
            Average::Mean => inverse(&mean(shares.iter().map(|&(w, e)| (w, e.0)))),
            Average::InverseMean => mean(shares.iter().map(|&(w, e)| (w, inverse(&e.0)))),
            Average::Sampled | Average::Subpixel => {
                let tau = mean(
                    shares
                        .iter()
                        .map(|&(w, e)| (w, tau(&rotated(&frame, &e.0)))),
                );
                let eps = rotated(&transpose(&frame), &tau_inverse(&tau));
                inverse(&eps)
            }
        }
    }
}

/// The share of a box of `size` about the origin on the side of the plane n·x + d = 0 that n
/// points away from (n·x + d < 0), n a unit vector: exact.
///
/// With tᵢ ∈ [0, 1] across the box and aᵢ = |nᵢ| sizeᵢ, it is the volume of Σ aᵢtᵢ < s,
/// s = Σaᵢ/2 − d. Along the axis of the smallest a, the area of the rest's cut is piecewise
/// quadratic between the sums of the rest's a, so two-point Gauss–Legendre on each piece is
/// exact, with no division by a small a.
pub(crate) fn fill(n: [f64; 3], d: f64, size: [f64; 3]) -> f64 {
    let mut a: Vec<f64> = (0..3).map(|i| n[i].abs() * size[i]).collect();
    let total: f64 = a.iter().sum();
    let s = 0.5 * total - d;
    if s <= 0.0 {
        return 0.0;
    }
    if s >= total {
        return 1.0;
    }
    a.retain(|&v| v > 1e-14 * total);
    a.sort_by(f64::total_cmp);
    // the 1D and 2D cuts, the larger a last
    let line = |s: f64, a: f64| (s / a).clamp(0.0, 1.0);
    let area = |s: f64, a: f64, b: f64| -> f64 {
        let half = |s: f64| {
            if s <= 0.0 {
                0.0
            } else if s <= a {
                s * s / (2.0 * a * b)
            } else {
                (2.0 * s - a) / (2.0 * b)
            }
        };
        if 2.0 * s <= a + b {
            half(s)
        } else {
            1.0 - half(a + b - s)
        }
    };
    match a.len() {
        1 => line(s, a[0]),
        2 => area(s, a[0], a[1]),
        _ => {
            // ∫₀¹ area(s − a₀t) dt, split where s − a₀t crosses 0, a₁, a₂ or a₁ + a₂
            let mut cuts = vec![0.0, 1.0];
            for b in [0.0, a[1], a[2], a[1] + a[2]] {
                let t = (s - b) / a[0];
                if t > 0.0 && t < 1.0 {
                    cuts.push(t);
                }
            }
            cuts.sort_by(f64::total_cmp);
            let g = 0.5 / 3f64.sqrt();
            cuts.windows(2)
                .map(|w| {
                    let (mid, half) = (0.5 * (w[0] + w[1]), w[1] - w[0]);
                    let at = |t: f64| area(s - a[0] * t, a[1], a[2]);
                    0.5 * half * (at(mid - g * half) + at(mid + g * half))
                })
                .sum()
        }
    }
}

/// A frame whose first row is the unit vector n, its rows orthonormal.
fn frame(n: [f64; 3]) -> Matrix {
    // the grid's axis least along n, crossed with n
    let least = (0..3)
        .min_by(|&i, &j| n[i].abs().total_cmp(&n[j].abs()))
        .unwrap();
    let mut e = [0.0; 3];
    e[least] = 1.0;
    let t = cross(n, e);
    let t = t.map(|v| v / norm(t));
    [n, t, cross(n, t)]
}

/// R M Rᵀ.
fn rotated(r: &Matrix, m: &Matrix) -> Matrix {
    std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            (0..3)
                .map(|k| (0..3).map(|l| r[i][k] * m[k][l] * r[j][l]).sum::<f64>())
                .sum()
        })
    })
}

/// Kottke et al.'s Eq. 4.
fn tau(e: &Matrix) -> Matrix {
    let mut t = [[0.0; 3]; 3];
    t[0][0] = -1.0 / e[0][0];
    for j in 1..3 {
        t[0][j] = e[0][j] / e[0][0];
        t[j][0] = e[j][0] / e[0][0];
    }
    for i in 1..3 {
        for j in 1..3 {
            t[i][j] = e[i][j] - e[i][0] * e[0][j] / e[0][0];
        }
    }
    t
}

/// Its inverse, their Eq. 23.
fn tau_inverse(t: &Matrix) -> Matrix {
    let mut e = [[0.0; 3]; 3];
    e[0][0] = -1.0 / t[0][0];
    for j in 1..3 {
        e[0][j] = -t[0][j] / t[0][0];
        e[j][0] = -t[j][0] / t[0][0];
    }
    for i in 1..3 {
        for j in 1..3 {
            e[i][j] = t[i][j] - t[i][0] * t[0][j] / t[0][0];
        }
    }
    e
}

fn mean(terms: impl Iterator<Item = (f64, Matrix)>) -> Matrix {
    let mut m = [[0.0; 3]; 3];
    for (w, t) in terms {
        for i in 0..3 {
            for j in 0..3 {
                m[i][j] += w * t[i][j];
            }
        }
    }
    m
}

fn determinant(m: &Matrix) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

/// The inverse of a symmetric matrix, by its cofactors, exactly symmetric.
fn inverse(m: &Matrix) -> Matrix {
    let det = determinant(m);
    let c = |i: usize, j: usize| {
        let (r0, r1) = ((i + 1) % 3, (i + 2) % 3);
        let (c0, c1) = ((j + 1) % 3, (j + 2) % 3);
        m[r0][c0] * m[r1][c1] - m[r0][c1] * m[r1][c0]
    };
    // the inverse is the transposed cofactors over the determinant
    symmetrized(std::array::from_fn(|i| {
        std::array::from_fn(|j| c(j, i) / det)
    }))
}

fn symmetrized(m: Matrix) -> Matrix {
    std::array::from_fn(|i| std::array::from_fn(|j| 0.5 * (m[i][j] + m[j][i])))
}

fn transpose(m: &Matrix) -> Matrix {
    std::array::from_fn(|i| std::array::from_fn(|j| m[j][i]))
}

fn orthonormal(axes: &Matrix) -> bool {
    (0..3).all(|i| {
        (0..3).all(|j| {
            let expected = if i == j { 1.0 } else { 0.0 };
            (dot(axes[i], axes[j]) - expected).abs() <= 1e-9
        })
    })
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// What a box of the structure holds ([`Structure::local`]).
enum Local {
    Uniform(Permittivity),
    /// One body's surface, as a plane: the signed distance from the box's centre (negative
    /// inside) and the outward normal; the body's medium and the one beneath it.
    Plane {
        d: f64,
        n: [f64; 3],
        inside: Permittivity,
        beneath: Permittivity,
    },
    /// Two surfaces or more.
    Many,
}

/// The eight triplets at a node, numbered sx + 2sy + 4sz: s = 0 where the triplet's value of
/// that component is on the edge from the node the positive way, 1 the negative way.
const TRIPLETS: usize = 8;

/// The side (0 positive, 1 negative) of triplet `t`'s value of `component`.
fn side(t: usize, component: usize) -> usize {
    (t >> component) & 1
}

/// The inverse of a 3 × 3 matrix by its cofactors, if its determinant is non-zero and finite.
fn inverse_general(m: &Matrix) -> Option<Matrix> {
    let det = determinant(m);
    let scale = m.iter().flatten().fold(0.0f64, |a, v| a.max(v.abs()));
    if !(det.is_finite() && det.abs() > 1e-12 * scale * scale * scale) {
        return None;
    }
    let c = |i: usize, j: usize| {
        let (r0, r1) = ((i + 1) % 3, (i + 2) % 3);
        let (c0, c1) = ((j + 1) % 3, (j + 2) % 3);
        m[r0][c0] * m[r1][c1] - m[r0][c1] * m[r1][c0]
    };
    Some(std::array::from_fn(|i| {
        std::array::from_fn(|j| c(j, i) / det)
    }))
}

fn product(a: &Matrix, b: &Matrix) -> Matrix {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum()))
}

/// Sylvester's criterion for a symmetric matrix.
fn positive_definite(m: &Matrix) -> bool {
    m[0][0] > 0.0 && m[0][0] * m[1][1] - m[0][1] * m[1][0] > 0.0 && determinant(m) > 0.0
}

/// The eight triplets' tensors (ε̃⁻¹ for each) at the node `p` (µm) of a grid of steps `h`, by
/// Werner, Bauer and Cary's 2013 scheme, `symmetric`; or Bauer, Werner and Cary's 2011 tensor
/// as it is, not symmetric, when not (for the checks: second order, but not stable).
fn triplets(structure: &Structure, p: [f64; 3], h: [f64; 3], symmetric: bool) -> [Matrix; 8] {
    // Kottke et al.'s average over the node's cell: where two surfaces come near, or where
    // Bauer et al.'s tensor fails (Werner et al. 2013, Sec. 7, step 6)
    let fallback = || [structure.inverse(p, h, Average::Subpixel); TRIPLETS];
    // every edge and dual face of the node's triplets lies within a box of 2h about it
    match structure.local(p, h.map(|v| 2.0 * v)) {
        Local::Uniform(eps) => [inverse(&eps.0); TRIPLETS],
        Local::Many => fallback(),
        Local::Plane {
            d,
            n,
            inside,
            beneath,
        } => bauer(n, d, h, &inside.0, &beneath.0, symmetric).unwrap_or_else(fallback),
    }
}

/// Bauer, Werner and Cary's tensors (their Eqs. 27 to 44, as Werner et al. 2013 put them, Eqs.
/// 15 to 20) at a node `d` from the plane of normal `n` (negative inside), on a grid of steps `h`:
/// for each triplet, E = Λ_C Λ_P⁻¹ D with F = (D_n, E_t, E_s), continuous across the plane, and
/// E = C F, D = P F in each medium, C = 1 + n(n − εn)ᵀ/(nᵀεn) and P = εC; the rows of Λ_C are
/// C's rows averaged along each value's edge, those of Λ_P P's over its dual face. Made
/// symmetric, `symmetric`; `None` where Λ_P is singular or the symmetric tensor isn't positive.
fn bauer(
    n: [f64; 3],
    d: f64,
    h: [f64; 3],
    inside: &Matrix,
    beneath: &Matrix,
    symmetric: bool,
) -> Option<[Matrix; 8]> {
    // the share inside of each edge and dual face, each way along each axis: both centred on
    // the value of E, half a step from the node
    let mut line = [[0.0; 2]; 3];
    let mut face = [[0.0; 2]; 3];
    for a in 0..3 {
        for (s, sign) in [1.0, -1.0].into_iter().enumerate() {
            let centre = d + sign * 0.5 * h[a] * n[a];
            let mut edge = [0.0; 3];
            edge[a] = h[a];
            line[a][s] = fill(n, centre, edge);
            let mut dual = h;
            dual[a] = 0.0;
            face[a][s] = fill(n, centre, dual);
        }
    }
    let shares = || line.iter().chain(&face).flatten();
    if shares().all(|&f| f == 1.0) {
        return Some([inverse(inside); TRIPLETS]);
    }
    if shares().all(|&f| f == 0.0) {
        return Some([inverse(beneath); TRIPLETS]);
    }
    let mixed = |eps: &Matrix| -> (Matrix, Matrix) {
        let en: [f64; 3] = std::array::from_fn(|i| dot(eps[i], n));
        let nen = dot(n, en);
        let c: Matrix = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                let identity = if i == j { 1.0 } else { 0.0 };
                identity + n[i] * (n[j] - en[j]) / nen
            })
        });
        (c, product(eps, &c))
    };
    let ((c1, p1), (c2, p2)) = (mixed(inside), mixed(beneath));
    let mut out = [[[0.0; 3]; 3]; TRIPLETS];
    for (t, k) in out.iter_mut().enumerate() {
        let lc: Matrix = std::array::from_fn(|r| {
            let f = line[r][side(t, r)];
            std::array::from_fn(|j| f * c1[r][j] + (1.0 - f) * c2[r][j])
        });
        let lp: Matrix = std::array::from_fn(|r| {
            let f = face[r][side(t, r)];
            std::array::from_fn(|j| f * p1[r][j] + (1.0 - f) * p2[r][j])
        });
        let accurate = product(&lc, &inverse_general(&lp)?);
        *k = if symmetric {
            let k = symmetrized(accurate);
            if !positive_definite(&k) {
                return None;
            }
            k
        } else {
            accurate
        };
    }
    Some(out)
}

/// A node's part of ε̃⁻¹ on the grid from its triplets' tensors, an eighth of each: for each
/// component c and side s_c of the node, the diagonal entry (6, numbered 2c + s_c), then for
/// each other component a (its slot in [`Axis::others`]) and side s_a, the entry coupling the
/// value of c on side s_c to that of a on side s_a (24, numbered 6 + 4(2c + slot) + 2s_c + s_a).
fn record(k: &[Matrix; 8]) -> [f64; 30] {
    let mut out = [0.0; 30];
    for c in 0..3 {
        let (a, b) = Axis::ALL[c].others();
        for sc in 0..2 {
            let ts = || (0..TRIPLETS).filter(move |&t| side(t, c) == sc);
            out[2 * c + sc] = 0.125 * ts().map(|t| k[t][c][c]).sum::<f64>();
            for (slot, other) in [a, b].into_iter().enumerate() {
                let o = other.index();
                for sa in 0..2 {
                    let sum: f64 = ts().filter(|&t| side(t, o) == sa).map(|t| k[t][c][o]).sum();
                    out[6 + 4 * (2 * c + slot) + 2 * sc + sa] = 0.125 * sum;
                }
            }
        }
    }
    out
}

impl Simulation {
    /// The problem on `grid` with the permittivity of `structure`, each value of E's taken by
    /// `smoothing`, inside `boundaries`, stepped at the Courant number `courant` (0 < C ≤ 1).
    ///
    /// Where the smoothed tensor couples E's components (an interface not along the grid's
    /// axes, or an anisotropic medium whose principal axes aren't the grid's), the simulation
    /// steps D and finds E from it, and E set by [`Simulation::e_mut`] before a step enters
    /// only H̃'s next update. Such a medium takes no conductivity and no dispersive medium.
    ///
    /// # Errors
    ///
    /// As [`Simulation::new`], for a diameter that isn't positive and finite, and for a smoothed
    /// tensor that couples E's components with a Bloch boundary of k ≠ 0 (not supported), and
    /// for [`Coupling::Nodes`] where its ε̃⁻¹ on the grid isn't positive definite: a high
    /// contrast across an interface oblique to the grid, ε = 50 against vacuum, where the run
    /// would grow without bound (issue #209).
    pub fn smoothed(
        grid: Grid3d,
        structure: &Structure,
        smoothing: Smoothing,
        boundaries: Boundaries,
        courant: f64,
    ) -> Result<Simulation> {
        let s = Simulation::smoothed_unchecked(grid, structure, smoothing, boundaries, courant)?;
        // the triplets' ε̃⁻¹ is positive definite by construction
        let checked = smoothing.coupling == Coupling::Nodes;
        if checked && s.anisotropic.as_ref().is_some_and(|a| !a.positive()) {
            return Err(invalid(
                "the smoothed ε⁻¹ with its off-diagonal entries at the nodes isn't positive \
                 definite here, so the leapfrog's energy isn't positive and the run would grow \
                 without bound: a contrast this high across an interface oblique to the grid is \
                 beyond Werner and Cary's 2007 scheme; Coupling::Triplets (Werner, Bauer and \
                 Cary 2013) is stable at any contrast",
            ));
        }
        Ok(s)
    }

    /// As [`Simulation::smoothed`], without its check that ε̃⁻¹ on the grid is positive
    /// definite: for the checks that show what it prevents.
    fn smoothed_unchecked(
        grid: Grid3d,
        structure: &Structure,
        smoothing: Smoothing,
        boundaries: Boundaries,
        courant: f64,
    ) -> Result<Simulation> {
        let Smoothing {
            average,
            diameter,
            coupling,
        } = smoothing;
        if !(diameter.is_finite() && diameter > 0.0) {
            return Err(invalid(format!(
                "a smoothing diameter must be positive and finite, got {diameter}"
            )));
        }
        if coupling == Coupling::Triplets {
            if average != Average::Subpixel || diameter != 1.0 {
                return Err(invalid(format!(
                    "Coupling::Triplets takes its own average: it needs Average::Subpixel and a \
                     diameter of 1, got {average:?} and {diameter}"
                )));
            }
            return Simulation::triplets(grid, structure, boundaries, courant, true);
        }
        let size = [grid.dx, grid.dy, grid.dz].map(|h| h * diameter);
        let at = |r: usize| {
            (
                r % grid.nx,
                (r / grid.nx) % grid.ny,
                r / (grid.nx * grid.ny),
            )
        };
        // each value's row of ε̃⁻¹ over the cell about it
        let mut rows: [Vec<[f64; 3]>; 3] = [Vec::new(), Vec::new(), Vec::new()];
        let s = Simulation::with_permittivity(grid, boundaries, courant, |grid| {
            rows = Axis::ALL.map(|c| {
                (0..grid.cells())
                    .into_par_iter()
                    .map(|r| structure.inverse(grid.e_position(c, at(r)), size, average)[c.index()])
                    .collect()
            });
            Ok(std::array::from_fn(|c| {
                rows[c].iter().map(|row| 1.0 / row[c]).collect()
            }))
        })?;
        let n = grid.cells();
        // the off-diagonal entries: (ε̃⁻¹)_yz, _zx and _xy at the nodes, over the cells about
        // them, or (ε̃⁻¹)_ca at each value of E_c
        let off = match coupling {
            Coupling::Nodes => {
                let nodes: Vec<[f64; 3]> = (0..n)
                    .into_par_iter()
                    .map(|r| {
                        let (i, j, k) = at(r);
                        let p = [
                            grid.node(Axis::X, i),
                            grid.node(Axis::Y, j),
                            grid.node(Axis::Z, k),
                        ];
                        let m = structure.inverse(p, size, average);
                        [m[1][2], m[2][0], m[0][1]]
                    })
                    .collect();
                Off::Nodes(std::array::from_fn(|a| {
                    nodes.iter().map(|m| m[a]).collect()
                }))
            }
            Coupling::Points => Off::Points(rows.clone()),
            Coupling::Triplets => unreachable!("taken by Simulation::triplets"),
        };
        let diagonal = std::array::from_fn(|c| rows[c].iter().map(|row| row[c]).collect());
        s.couple(off, diagonal)
    }

    /// As [`Simulation::smoothed`] with [`Coupling::Triplets`], or with Bauer et al.'s 2011
    /// tensors as they are when not `symmetric` (not stable: for the checks).
    pub(crate) fn triplets(
        grid: Grid3d,
        structure: &Structure,
        boundaries: Boundaries,
        courant: f64,
        symmetric: bool,
    ) -> Result<Simulation> {
        let n = grid.cells();
        let h = [grid.dx, grid.dy, grid.dz];
        let at = |r: usize| {
            [
                r % grid.nx,
                (r / grid.nx) % grid.ny,
                r / (grid.nx * grid.ny),
            ]
        };
        let node = |m: [usize; 3]| {
            [
                grid.node(Axis::X, m[0]),
                grid.node(Axis::Y, m[1]),
                grid.node(Axis::Z, m[2]),
            ]
        };
        let record_at = |m: [usize; 3]| record(&triplets(structure, node(m), h, symmetric));
        // every node's record, the same ones stored once, a chunk of nodes at a time
        let mut table: Vec<[f64; 30]> = Vec::new();
        let mut seen: std::collections::HashMap<[u64; 30], u32> = Default::default();
        let mut index = vec![0u32; n];
        const CHUNK: usize = 1 << 16;
        for start in (0..n).step_by(CHUNK) {
            let records: Vec<[f64; 30]> = (start..(start + CHUNK).min(n))
                .into_par_iter()
                .map(|r| record_at(at(r)))
                .collect();
            for (r, record) in (start..).zip(records) {
                let key = record.map(f64::to_bits);
                index[r] = *seen.entry(key).or_insert_with(|| {
                    table.push(record);
                    (table.len() - 1) as u32
                });
            }
        }
        // each value of E's diagonal entry: from the node it starts at and the one it ends at,
        // which beyond a side that isn't periodic is outside the grid (a wall's node, with no
        // values of the other components to couple to)
        let stride = [1, grid.nx, grid.nx * grid.ny];
        let diagonal: [Vec<f64>; 3] = std::array::from_fn(|c| {
            let axis = Axis::ALL[c];
            let count = grid.n(axis);
            (0..n)
                .into_par_iter()
                .map(|r| {
                    let m = at(r);
                    let own = table[index[r] as usize][2 * c];
                    let far = if m[c] + 1 < count {
                        table[index[r + stride[c]] as usize][2 * c + 1]
                    } else if boundaries.periodic(axis) {
                        table[index[r + stride[c] - count * stride[c]] as usize][2 * c + 1]
                    } else {
                        let mut beyond = m;
                        beyond[c] += 1;
                        record_at(beyond)[2 * c + 1]
                    };
                    own + far
                })
                .collect()
        });
        let s = Simulation::with_permittivity(grid, boundaries, courant, |_| {
            Ok(std::array::from_fn(|c| {
                diagonal[c].iter().map(|v| 1.0 / v).collect()
            }))
        })?;
        let table = table
            .iter()
            .map(|record| std::array::from_fn(|q| record[6 + q]))
            .collect();
        s.couple(Off::Triplets { index, table }, diagonal)
    }

    /// The simulation stepping D and finding E = ε̃⁻¹D, ε̃⁻¹'s diagonal entries `diagonal` and
    /// its others `off`, if any of those isn't zero; as it is otherwise.
    fn couple(mut self, off: Off, diagonal: [Vec<f64>; 3]) -> Result<Simulation> {
        let s = &mut self;
        let grid = s.grid;
        let n = grid.cells();
        let at = |r: usize| {
            (
                r % grid.nx,
                (r / grid.nx) % grid.ny,
                r / (grid.nx * grid.ny),
            )
        };
        let any = match &off {
            Off::Nodes(w) => w.iter().flatten().any(|&v| v != 0.0),
            Off::Points(rows) => (0..3).any(|c| {
                rows[c]
                    .iter()
                    .any(|row| (0..3).any(|a| a != c && row[a] != 0.0))
            }),
            Off::Triplets { index, table } => {
                let used: Vec<bool> = table.iter().map(|w| w.iter().any(|&v| v != 0.0)).collect();
                index.iter().any(|&i| used[i as usize])
            }
        };
        if !any {
            return Ok(self);
        }
        if s.bloch.is_some() {
            return Err(invalid(
                "a permittivity that couples E's components with a Bloch phase isn't supported: \
                 D's coupling across the Bloch sides would need the phase",
            ));
        }
        let forward = s.offsets(true);
        // the values of E an off-diagonal entry reaches
        let coupled = Axis::ALL.map(|c| {
            let ci = c.index();
            (0..n)
                .map(|r| {
                    let (i, j, k) = at(r);
                    let next = forward[ci][[i, j, k][ci]].map(|o| (r as isize + o) as usize);
                    let (a, b) = c.others();
                    [a, b].into_iter().enumerate().any(|(slot, other)| {
                        let oi = other.index();
                        match &off {
                            Off::Nodes(w) => {
                                let w = &w[3 - ci - oi];
                                w[r] != 0.0 || next.is_some_and(|q| w[q] != 0.0)
                            }
                            Off::Points(rows) => rows[ci][r][oi] != 0.0,
                            Off::Triplets { index, table } => {
                                [(Some(r), 0), (next, 1)].into_iter().any(|(node, sc)| {
                                    node.is_some_and(|node| {
                                        let w = &table[index[node] as usize];
                                        let k = 4 * (2 * ci + slot) + 2 * sc;
                                        w[k] != 0.0 || w[k + 1] != 0.0
                                    })
                                })
                            }
                        }
                    })
                })
                .collect()
        });
        let anisotropic = Anisotropic {
            d: [vec![0.0; n], vec![0.0; n], vec![0.0; n]],
            cb: s.cb.clone().map(|c| {
                c.iter()
                    .map(|&v| if v == 0.0 { 0.0 } else { s.dt })
                    .collect()
            }),
            diagonal,
            off,
            coupled,
            forward,
            back: s.offsets(false),
            grid,
        };
        s.anisotropic = Some(Box::new(anisotropic));
        Ok(self)
    }
}

/// ε̃⁻¹'s off-diagonal entries, by [`Coupling`].
#[derive(Clone, Debug)]
enum Off {
    /// (ε̃⁻¹)_yz, _zx and _xy at the nodes, numbered by the axis neither is along.
    Nodes([Vec<f64>; 3]),
    /// Each value of E_c's row of ε̃⁻¹, (ε̃⁻¹)_ca for each a.
    Points([Vec<[f64; 3]>; 3]),
    /// Each node's off-diagonal entries from its triplets, as [`record`] numbers them less 6,
    /// the same ones stored once: `table[index[node]]`.
    Triplets {
        index: Vec<u32>,
        table: Vec<[f64; 24]>,
    },
}

/// A permittivity tensor that couples E's components: D, stepped as E would be in vacuum, and
/// ε̃⁻¹ to find E from it.
#[derive(Clone, Debug)]
pub(super) struct Anisotropic {
    d: [Vec<f64>; 3],
    /// D's update coefficients, Δt, or 0 where E is held at zero.
    cb: [Vec<f64>; 3],
    /// (ε̃⁻¹)_cc at each value of E_c.
    diagonal: [Vec<f64>; 3],
    off: Off,
    /// The values of each component of E that an off-diagonal entry reaches.
    coupled: [Vec<bool>; 3],
    /// [`Simulation::offsets`], forward and back.
    forward: [Vec<Option<isize>>; 3],
    back: [Vec<Option<isize>>; 3],
    grid: Grid3d,
}

impl Anisotropic {
    /// Before E's update: D and its coefficients take E's and its coefficients' places.
    pub(super) fn begin(&mut self, e: &mut [Vec<f64>; 3], cb: &mut [Vec<f64>; 3]) {
        std::mem::swap(e, &mut self.d);
        std::mem::swap(cb, &mut self.cb);
    }

    /// After it, its sources and its corrections: back in their places, and E = ε̃⁻¹D, zero
    /// where it is held there.
    pub(super) fn end(&mut self, e: &mut [Vec<f64>; 3], cb: &mut [Vec<f64>; 3]) {
        std::mem::swap(e, &mut self.d);
        std::mem::swap(cb, &mut self.cb);
        let g = self.grid;
        let plane = g.nx * g.ny;
        let (d, diagonal, off, coupled) = (&self.d, &self.diagonal, &self.off, &self.coupled);
        let (forward, back) = (&self.forward, &self.back);
        for c in Axis::ALL {
            let ci = c.index();
            let (a, b) = c.others();
            let held = &cb[ci];
            e[ci]
                .par_chunks_mut(plane)
                .enumerate()
                .for_each(|(k, values)| {
                    for j in 0..g.ny {
                        for i in 0..g.nx {
                            let m = [i, j, k];
                            let r = k * plane + j * g.nx + i;
                            let v = &mut values[j * g.nx + i];
                            if held[r] == 0.0 {
                                *v = 0.0;
                                continue;
                            }
                            *v = diagonal[ci][r] * d[ci][r];
                            if !coupled[ci][r] {
                                continue;
                            }
                            // the nodes at either end of this value's edge, and D_a on either
                            // side of each
                            let next = forward[ci][m[ci]].map(|o| (r as isize + o) as usize);
                            for (slot, other) in [a, b].into_iter().enumerate() {
                                let oi = other.index();
                                let pair = |node: usize| {
                                    d[oi][node]
                                        + back[oi][m[oi]]
                                            .map_or(0.0, |o| d[oi][(node as isize + o) as usize])
                                };
                                match off {
                                    Off::Nodes(w) => {
                                        let w = &w[3 - ci - oi];
                                        for node in [Some(r), next].into_iter().flatten() {
                                            if w[node] != 0.0 {
                                                *v += 0.25 * w[node] * pair(node);
                                            }
                                        }
                                    }
                                    Off::Points(rows) => {
                                        let sum = pair(r) + next.map_or(0.0, pair);
                                        *v += 0.25 * rows[ci][r][oi] * sum;
                                    }
                                    Off::Triplets { index, table } => {
                                        for (node, sc) in [(Some(r), 0), (next, 1)] {
                                            let Some(node) = node else { continue };
                                            let w = &table[index[node] as usize];
                                            let k = 4 * (2 * ci + slot) + 2 * sc;
                                            if w[k] != 0.0 {
                                                *v += w[k] * d[oi][node];
                                            }
                                            if w[k + 1] != 0.0
                                                && let Some(o) = back[oi][m[oi]]
                                            {
                                                *v +=
                                                    w[k + 1] * d[oi][(node as isize + o) as usize];
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                });
        }
    }

    /// Whether ε̃⁻¹ on the grid, E = ε̃⁻¹D, is positive definite where it couples E's components:
    /// the leapfrog's energy ½ Σ E·D + ½ Σ H̃·H̃ is then positive, and the scheme stable below
    /// a Courant number near 1. Werner and Cary's [`Coupling::Nodes`] is symmetric but not
    /// positive at every contrast: an interface between ε = 50 and vacuum oblique to the grid
    /// has a negative eigenvalue, about −0.005 (docs/methods/fdtd.md). The coupled values are
    /// their own block of ε̃⁻¹, a thin band about the interfaces, and its Cholesky factors
    /// exist exactly when it is positive definite. Values held at zero are left out: what
    /// remains is a principal block, positive when the whole is.
    fn positive(&self) -> bool {
        if matches!(self.off, Off::Points(_)) {
            return true;
        }
        let n = self.grid.cells();
        let at = |r: usize| {
            let g = self.grid;
            [r % g.nx, (r / g.nx) % g.ny, r / (g.nx * g.ny)]
        };
        // each coupled value's place in the block
        let mut size = 0;
        let place: [Vec<usize>; 3] = std::array::from_fn(|c| {
            (self.coupled[c].iter().zip(&self.cb[c]))
                .map(|(&coupled, &cb)| {
                    if coupled && cb != 0.0 {
                        size += 1;
                        size - 1
                    } else {
                        usize::MAX
                    }
                })
                .collect()
        });
        if size == 0 {
            return true;
        }
        // the lower triangle, row by row, each row's entries summed by column
        let mut triplets = Vec::new();
        let mut row: Vec<(usize, f64)> = Vec::new();
        for c in Axis::ALL {
            let ci = c.index();
            let (a, b) = c.others();
            for r in 0..n {
                let p = place[ci][r];
                if p == usize::MAX {
                    continue;
                }
                let m = at(r);
                row.clear();
                row.push((p, self.diagonal[ci][r]));
                let next = self.forward[ci][m[ci]].map(|o| (r as isize + o) as usize);
                for (slot, other) in [a, b].into_iter().enumerate() {
                    let oi = other.index();
                    for (node, sc) in [(Some(r), 0), (next, 1)] {
                        let Some(node) = node else { continue };
                        let below = self.back[oi][m[oi]].map(|o| (node as isize + o) as usize);
                        for (q, sa) in [(Some(node), 0), (below, 1)] {
                            let weight = match &self.off {
                                Off::Nodes(w) => 0.25 * w[3 - ci - oi][node],
                                Off::Triplets { index, table } => {
                                    table[index[node] as usize][4 * (2 * ci + slot) + 2 * sc + sa]
                                }
                                Off::Points(_) => unreachable!("not symmetric"),
                            };
                            if let Some(q) = q
                                && weight != 0.0
                                && place[oi][q] != usize::MAX
                            {
                                row.push((place[oi][q], weight));
                            }
                        }
                    }
                }
                row.sort_by_key(|&(q, _)| q);
                let mut k = 0;
                while k < row.len() {
                    let (q, mut v) = row[k];
                    k += 1;
                    while k < row.len() && row[k].0 == q {
                        v += row[k].1;
                        k += 1;
                    }
                    if q <= p {
                        triplets.push(Triplet::new(p, q, v));
                    }
                }
            }
        }
        SparseColMat::<usize, f64>::try_new_from_triplets(size, size, &triplets)
            .is_ok_and(|block| block.sp_cholesky(Side::Lower).is_ok())
    }

    /// E's `component` held at zero at value `r`, a conductor's.
    pub(super) fn hold(&mut self, component: usize, r: usize) {
        self.cb[component][r] = 0.0;
        self.d[component][r] = 0.0;
    }

    /// Σ E·D.
    pub(super) fn dot(&self, e: &[Vec<f64>; 3]) -> f64 {
        (0..3)
            .map(|c| e[c].iter().zip(&self.d[c]).map(|(x, y)| x * y).sum::<f64>())
            .sum()
    }
}

pub(crate) mod checks;
#[cfg(test)]
mod tests;
