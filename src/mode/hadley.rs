//! Full-vector modes by Hadley's high-accuracy finite-difference equations.
//!
//! G. R. Hadley, "High-accuracy finite-difference equations for dielectric waveguide analysis I:
//! uniform regions and dielectric interfaces", J. Lightwave Technol. 20, 1210 (2002),
//! [doi:10.1109/JLT.2002.800361](https://doi.org/10.1109/JLT.2002.800361), and "… II:
//! dielectric corners", J. Lightwave Technol. 20, 1219 (2002),
//! [doi:10.1109/JLT.2002.800371](https://doi.org/10.1109/JLT.2002.800371).
//!
//! The unknowns are the same as [`crate::mode::vector`]'s: H_x and H_y at the nodes of a
//! rectilinear grid, the permittivity uniform in each cell, so every dielectric boundary lies on
//! grid lines. Each node's equation reads the nine-point stencil around it, but instead of
//! Taylor series it is built from the exact local solutions of the Helmholtz equation
//! ∇²H + k²(ε − ε̄)H = 0, ε̄ = n_eff² (Hadley I, Eq. 1), in Bessel functions of ξr with
//! ξ² = k²(ε − ε̄):
//!
//! - **uniform** nodes (all four cells alike): Hadley I, Eqs. (7)–(9), a truncation error of
//!   sixth order in the spacing on a square grid (fourth otherwise);
//! - **interface** nodes (two pairs of equal cells): the component normal to the interface by
//!   Hadley I, Eqs. (20)–(24), uncoupled, fifth order; the tangential one by Eq. (43), with the
//!   A's from Eqs. (31), (32), (23), (24), coupled to the normal component through its
//!   derivative along the interface. Hadley derives them for a horizontal interface; a vertical
//!   one is the same with x and y exchanged, H_x with H_y (the interface condition, his Eq. 25,
//!   is symmetric under that exchange);
//! - **corner** nodes (anything else, Hadley II, Fig. 3): Eq. (50) for H_y and Eq. (52) for H_x,
//!   with the coefficients of his Appendix, Eqs. (A1)–(A14). The basis adds the corner's
//!   fractional powers ν and μ, the two roots of Eq. (36) in [5/3, 7/3], and logarithmic terms;
//!   the equations are first order at the corner itself.
//!
//! The coefficients depend on ε̄ through ξ, so the eigenproblem M(ε̄) h = 0 is nonlinear. It is
//! solved by nonlinear inverse iteration (S. Güttel, F. Tisseur, "The nonlinear eigenvalue
//! problem", Acta Numerica 26, 1 (2017),
//! [doi:10.1017/S0962492917000034](https://doi.org/10.1017/S0962492917000034), Algorithm 4.7,
//! Eqs. 4.15–4.16, Newton's method on M(ε̄)h = 0 with a normalization), started from the
//! mode [`crate::mode::vector`] finds on the same grid, with dM/dε̄ by central differences.
//!
//! One misprint is corrected: Eqs. (50) and (52) print the term (θ sin θ + cos 2θ ln sin θ),
//! which they include to cancel A₂ and B₂ times the same term of Eq. (47), printed there as
//! (θ sin 2θ + cos 2θ ln sin θ). Eq. (47)'s is right: it comes from the second-order
//! logarithmic function r²(ln r cos 2θ − θ sin 2θ) (Eq. 24 with ν = 2), and exchanging x and y
//! (θ → π/2 − θ) turns it into the companion term ((π/2 − θ) sin 2θ − cos 2θ ln cos θ) that
//! Eqs. (48), (50) and (52) all print. (Taking the printed form changes the corner problems'
//! errors by up to a third, not their order.)
//!
//! Hadley's derivation needs a uniform grid (Δx and Δy may differ), real isotropic media and
//! no PML; [`modes`] checks all three. His corner equation is undefined where ε₁ε₃ = ε₂ε₄
//! (quadrants counterclockwise from the north-east): there the two corner exponents coincide.

use std::collections::HashMap;
use std::f64::consts::PI;

use faer::linalg::solvers::Solve;
use faer::sparse::{SparseColMat, Triplet};
use faer::{Mat, c64};

use crate::mode::vector::{self, Component, CrossSection, Numbering, VectorMode};
use crate::units::Wavelength;
use crate::{Error, Result};

const ZERO: c64 = c64::new(0.0, 0.0);
const ONE: c64 = c64::new(1.0, 0.0);

/// The largest Bessel argument |ξ| r the power series is trusted for: Hadley I notes that the
/// arguments stay below about 10 even on coarse grids.
const MAX_ARGUMENT: f64 = 12.0;

/// J_n(z), the Bessel function of the first kind of integer order n, by its power series
/// (accurate for the moderate |z| of a stencil, with no cancellation when z is imaginary).
fn bessel_j(n: u32, z: c64) -> c64 {
    let h = z * 0.5;
    let mut term = ONE;
    for k in 1..=n {
        term *= h / f64::from(k);
    }
    let q = -(h * h);
    let mut sum = term;
    let mut peak = term.norm();
    let mut k = 1.0;
    loop {
        term *= q / (k * (k + f64::from(n)));
        sum += term;
        peak = peak.max(term.norm());
        if term.norm() <= 1e-17 * peak || k > 300.0 {
            return sum;
        }
        k += 1.0;
    }
}

/// The grid and the modal permittivity the equations are built for.
#[derive(Clone, Copy, Debug)]
struct Grid {
    /// k², µm⁻².
    k2: f64,
    /// ε̄ = n_eff².
    eb: c64,
    /// The spacings, µm.
    hx: f64,
    hy: f64,
}

impl Grid {
    /// ξ = √(k²(ε − ε̄)) (Hadley I, Eq. 3), imaginary where ε < ε̄; the equations depend only
    /// on ξ², so the branch doesn't matter.
    fn xi(&self, eps: f64) -> c64 {
        (c64::new(self.k2 * eps, 0.0) - self.k2 * self.eb).sqrt()
    }
}

/// One term of a node's equation: the component it reads, the offset (di, dj) of the node it
/// reads, and the coefficient.
type Term = (Component, isize, isize, c64);

/// A term in an interface's own frame: offset along the interface, offset across it (towards
/// region 1), coefficient.
type Local = (isize, isize, c64);

/// Hadley I, Eqs. (7)–(9): the equation at a node inside a uniform region, for either
/// component. A and B cancel the c₂ and c₄ terms of the expansion.
fn uniform(eps: f64, c: Component, g: &Grid) -> Result<Vec<Term>> {
    let xi = g.xi(eps);
    let (hx, hy) = (g.hx, g.hy);
    let r = hx.hypot(hy);
    let t0 = hy.atan2(hx);
    let j = |n: u32, h: f64| bessel_j(n, xi * h);
    let (j2x, j4x, j2y, j4y) = (j(2, hx), j(4, hx), j(2, hy), j(4, hy));
    let p = j(2, r) * (2.0 * t0).cos();
    let q = j(4, r) * (4.0 * t0).cos();
    let det = j2x * j4y + j4x * j2y;
    // (8) and (9)
    let a = -2.0 * (p * j4y + j2y * q) / det;
    let b = 2.0 * (p * j4x - j2x * q) / det;
    if !(a.re.is_finite() && b.re.is_finite()) {
        return Err(Error::invalid(
            "Hadley's equations",
            format!(
                "the uniform equation is singular for eps {eps} at n_eff^2 {}",
                g.eb
            ),
        ));
    }
    // (7)
    let centre = -2.0 * a * j(0, hx) - 2.0 * b * j(0, hy) - 4.0 * j(0, r);
    Ok(vec![
        (c, 0, 0, centre),
        (c, 1, 0, a),
        (c, -1, 0, a),
        (c, 0, 1, b),
        (c, 0, -1, b),
        (c, 1, 1, ONE),
        (c, -1, 1, ONE),
        (c, 1, -1, ONE),
        (c, -1, -1, ONE),
    ])
}

/// Solves the 4 × 4 system m x = rhs by Gaussian elimination with partial pivoting, each row
/// first scaled to unit size (the rows of Hadley I's Eqs. 21–24 differ by powers of ξh).
fn solve4(mut m: [[c64; 4]; 4], mut rhs: [c64; 4]) -> Option<[c64; 4]> {
    for (row, b) in m.iter_mut().zip(rhs.iter_mut()) {
        let s = row.iter().map(|v| v.norm()).fold(0.0, f64::max);
        if s == 0.0 || !s.is_finite() {
            return None;
        }
        for v in row.iter_mut() {
            *v /= s;
        }
        *b /= s;
    }
    for col in 0..4 {
        let pivot = (col..4).max_by(|&a, &b| m[a][col].norm().total_cmp(&m[b][col].norm()))?;
        if m[pivot][col].norm() == 0.0 {
            return None;
        }
        m.swap(col, pivot);
        rhs.swap(col, pivot);
        let (top, below) = m.split_at_mut(col + 1);
        let pivot_row = top[col];
        for (offset, row) in below.iter_mut().enumerate() {
            let f = row[col] / pivot_row[col];
            for (v, p) in row.iter_mut().zip(&pivot_row).skip(col) {
                *v -= f * p;
            }
            let v = rhs[col];
            rhs[col + 1 + offset] -= f * v;
        }
    }
    let mut x = [ZERO; 4];
    for row in (0..4).rev() {
        let mut s = rhs[row];
        for k in row + 1..4 {
            s -= m[row][k] * x[k];
        }
        x[row] = s / m[row][row];
    }
    x.iter()
        .all(|v| v.re.is_finite() && v.im.is_finite())
        .then_some(x)
}

/// Hadley I's equation at a straight interface, in the interface's own frame: offsets along it
/// (spacing `ha`, his x) and across it towards region 1 (spacing `hb`, his y; region 1 has
/// permittivity `e1` and lies at positive offsets, region 2 has `e2`).
///
/// For the component normal to the interface (`normal`), Eqs. (20)–(24); for the tangential
/// one, Eq. (43), whose A's come from Eqs. (31), (32), (23) and (24) and which also reads the
/// normal component through f₁, f₂ and g₁–g₆ (Eqs. 39–42 and those after 43). Returns the
/// terms on the same component and those on the other one.
fn interface(
    e1: f64,
    e2: f64,
    ha: f64,
    hb: f64,
    normal: bool,
    g: &Grid,
) -> Result<(Vec<Local>, Vec<Local>)> {
    let (x1, x2) = (g.xi(e1), g.xi(e2));
    let al = x1 / x2;
    let (al2, al3) = (al * al, al * al * al);
    let al4 = al2 * al2;
    let be = ONE - al2;
    let (dx, dy) = (ha, hb);
    let r = dx.hypot(dy);
    let t0 = dy.atan2(dx);
    let s = |n: f64| (n * t0).sin();
    let c = |n: f64| (n * t0).cos();
    let j1 = |n: u32, h: f64| bessel_j(n, x1 * h);
    let j2 = |n: u32, h: f64| bessel_j(n, x2 * h);
    // R = ε₂/ε₁ in (31) and (32); 1 in (21) and (22)
    let rr = if normal { 1.0 } else { e2 / e1 };
    let m = [
        // (21) / (31): the d₃ terms
        [
            -j1(3, dy),
            rr * al3 * (j2(3, dy) - 3.0 * be * j2(5, dy)),
            2.0 * j1(3, r) * s(3.0),
            -2.0 * rr * al3 * (j2(3, r) * s(3.0) + 3.0 * be * j2(5, r) * s(5.0)),
        ],
        // (22) / (32): the d₁ terms
        [
            j1(1, dy),
            -rr * al * (j2(1, dy) - be * j2(3, dy) + be * (ONE - 2.0 * al2) * j2(5, dy)),
            2.0 * j1(1, r) * s(1.0),
            -2.0 * rr
                * al
                * (j2(1, r) * s(1.0)
                    + be * j2(3, r) * s(3.0)
                    + be * (ONE - 2.0 * al2) * j2(5, r) * s(5.0)),
        ],
        // (23): the c₂ terms
        [
            -j1(2, dy),
            -al2 * (j2(2, dy) - 4.0 * be * j2(4, dy) + 3.0 * be * (3.0 - 5.0 * al2) * j2(6, dy)),
            2.0 * j1(2, r) * c(2.0),
            2.0 * al2
                * (j2(2, r) * c(2.0)
                    + 4.0 * be * j2(4, r) * c(4.0)
                    + 3.0 * be * (3.0 - 5.0 * al2) * j2(6, r) * c(6.0)),
        ],
        // (24): the c₄ terms
        [
            j1(4, dy),
            al4 * (j2(4, dy) - 6.0 * be * j2(6, dy)),
            2.0 * j1(4, r) * c(4.0),
            2.0 * al4 * (j2(4, r) * c(4.0) + 6.0 * be * j2(6, r) * c(6.0)),
        ],
    ];
    let rhs = [ZERO, ZERO, -2.0 * j1(2, dx), -2.0 * j1(4, dx)];
    let [a1, a2, a3, a4] = solve4(m, rhs).ok_or_else(|| {
        Error::invalid(
            "Hadley's equations",
            format!("the interface equation between eps {e1} and {e2} is singular"),
        )
    })?;
    let poly6 = ONE - 8.0 * al2 + 10.0 * al4;
    let poly4 = ONE - 3.0 * al2;
    // (20), the same in (43)
    let same = vec![
        (1, 0, ONE),
        (-1, 0, ONE),
        (0, 0, -2.0 * j1(0, dx)),
        (0, 1, a1),
        (0, 0, -a1 * j1(0, dy)),
        (0, -1, a2),
        (
            0,
            0,
            a2 * (-j2(0, dy) + 2.0 * be * (j2(2, dy) - poly4 * j2(4, dy) + poly6 * j2(6, dy))),
        ),
        (1, 1, a3),
        (-1, 1, a3),
        (0, 0, -2.0 * a3 * j1(0, r)),
        (1, -1, a4),
        (-1, -1, a4),
        (
            0,
            0,
            a4 * (-2.0 * j2(0, r)
                - 4.0
                    * be
                    * (j2(2, r) * c(2.0) + poly4 * j2(4, r) * c(4.0) + poly6 * j2(6, r) * c(6.0))),
        ),
    ];
    let mut other = Vec::new();
    if !normal {
        // (39), (40)
        let lower = j2(2, r) - 2.0 * be * j2(4, r) * c(2.0);
        let f1 = 2.0 * al2 * j1(1, r) * c(1.0) * lower
            + 2.0 * al * j1(2, r) * (j2(1, r) * c(1.0) + 3.0 * be * j2(3, r) * c(3.0));
        let f2 = 2.0 * al2 * j1(3, r) * c(3.0) * lower + 2.0 * al3 * j1(2, r) * j2(3, r) * c(3.0);
        // after (42)
        let (j1x, j3x) = (j1(1, dx), j1(3, dx));
        let g1 = f1 - f2 * j1x / j3x;
        let g2 = ONE / (2.0 * j3x) + f2 * j1x / (2.0 * g1 * j3x * j3x);
        let g3 = al2 * j1x * lower / (g1 * j3x);
        let g4 = j1x * j1(2, r) / (g1 * j3x);
        // after (43), η = R − 1
        let eta = rr - 1.0;
        let p5 = 9.0 * al2 - 1.0 - 10.0 * al4;
        let g6 = al3 * eta * (j2(3, dy) + (5.0 * al2 - 3.0) * j2(5, dy)) * a2
            + 2.0 * al3 * eta * ((5.0 * al2 - 3.0) * j2(5, r) * s(5.0) - j2(3, r) * s(3.0)) * a4;
        let g5 = (al * eta * p5 * j2(5, dy)
            - al * eta * (2.0 * al2 - be) * j2(3, dy)
            - al * eta * j2(1, dy))
            * a2
            + (2.0 * al * eta * p5 * j2(5, r) * s(5.0)
                + 2.0 * al * eta * (2.0 * al2 - be) * j2(3, r) * s(3.0)
                - 2.0 * al * eta * j2(1, r) * s(1.0))
                * a4;
        // (43)'s last three terms, on the normal component
        let below = g5 * j1(2, r) / g1 - g6 * g4;
        let level = f2 * g5 / (2.0 * g1 * j3x) - g6 * g2;
        let above = al2 * g5 * lower / g1 - g6 * g3;
        other = vec![
            (1, -1, below),
            (-1, -1, -below),
            (1, 0, -level),
            (-1, 0, level),
            (1, 1, above),
            (-1, 1, -above),
        ];
    }
    if same
        .iter()
        .chain(&other)
        .any(|t| !(t.2.re.is_finite() && t.2.im.is_finite()))
    {
        return Err(Error::invalid(
            "Hadley's equations",
            format!("the interface equation between eps {e1} and {e2} is singular"),
        ));
    }
    Ok((same, other))
}

/// The roots ν ≥ μ of Hadley II's Eq. (36), sin²(πν/2) = −¼(ε₁₂ + ε₃₄)(ε₂₃ + ε₄₁) with
/// εᵢⱼ = (εᵢ − εⱼ)/(εᵢ + εⱼ), in [5/3, 7/3]: ν = 2 ± (2/π) asin √(…).
fn corner_exponents(e: [f64; 4]) -> Option<(f64, f64)> {
    let [e1, e2, e3, e4] = e;
    let eij = |a: f64, b: f64| (a - b) / (a + b);
    let s = -0.25 * (eij(e1, e2) + eij(e3, e4)) * (eij(e2, e3) + eij(e4, e1));
    if !(s > 1e-12 && s <= 0.25 + 1e-12) {
        return None;
    }
    let d = 2.0 / PI * s.min(0.25).sqrt().asin();
    Some((2.0 + d, 2.0 - d))
}

/// Hadley II's corner equations: Eq. (50) for H_y and Eq. (52) for H_x, at a node with
/// permittivities `e` = [ε₁, ε₂, ε₃, ε₄] in the quadrants north-east, north-west, south-west
/// and south-east (his Fig. 3), the operators of Eqs. (40)–(44) and the coefficients of his
/// Appendix, Eqs. (A1)–(A14).
fn corner(e: [f64; 4], c: Component, g: &Grid) -> Result<Vec<Term>> {
    use Component::{X, Y};
    let [e1, e2, e3, e4] = e;
    let (nu, mu) = corner_exponents(e).ok_or_else(|| {
        Error::invalid(
            "Hadley's equations",
            format!(
                "the corner equation is undefined at a corner of eps {e1}, {e2}, {e3}, {e4}: \
                 its exponents coincide (eps1 eps3 = eps2 eps4)"
            ),
        )
    })?;
    let (hx, hy) = (g.hx, g.hy);
    let r = hx.hypot(hy);
    let th = hy.atan2(hx);
    let e1234 = (e1 - e2) / (e1 + e2) + (e3 - e4) / (e3 + e4);
    let (c2, s2) = ((2.0 * th).cos(), (2.0 * th).sin());
    // (A3)–(A8) and (A12)–(A13) for one exponent p: (A or C, B or D, E or F, E' or F')
    let appendix = |p: f64| {
        let (cp, sp) = ((PI * p / 2.0).cos(), (PI * p / 2.0).sin());
        let a = 2.0 * (r.powf(p - 2.0) * (p * th).cos() + hy.powf(p - 2.0) * c2 * cp);
        let b = -4.0 * sp / e1234
            * (r.powf(p - 2.0) * (p * (th - PI / 2.0)).cos() - hx.powf(p - 2.0) * c2 * cp);
        let ee = -hx.powf(p - 2.0) - hy.powf(p - 2.0) * cp;
        let ep = 2.0 * sp / e1234 * (hy.powf(p - 2.0) + hx.powf(p - 2.0) * cp);
        (a, b, ee, ep)
    };
    let (a, b, ee, ep) = appendix(nu);
    let (cc, d, f, fp) = appendix(mu);
    let den = d * a - b * cc;
    // (A1), (A2), (A10), (A11)
    let ca2 = (d * ee - b * f) / den;
    let ca3 = (a * f - cc * ee) / den;
    let cb2 = (d * ep - b * fp) / den;
    let cb3 = (a * fp - cc * ep) / den;
    let sum = e1 + e2 + e3 + e4;
    // (A9), (A14)
    let ca1 = ((2.0 * ca2 * th.cos() / r + 1.0 / hx) * (e2 + e3 - e1 - e4)
        + 2.0 * ca3 * th.sin() / r * (e1 + e2 - e3 - e4))
        / sum;
    let cb1 = ((2.0 * cb3 * th.sin() / r + 1.0 / hy) * (e1 + e2 - e3 - e4)
        + 2.0 * cb2 * th.cos() / r * (e2 + e3 - e1 - e4))
        / sum;
    if ![ca1, ca2, ca3, cb1, cb2, cb3].iter().all(|v| v.is_finite()) {
        return Err(Error::invalid(
            "Hadley's equations",
            format!("the corner equation is singular at a corner of eps {e1}, {e2}, {e3}, {e4}"),
        ));
    }
    let xi = [g.xi(e1), g.xi(e2), g.xi(e3), g.xi(e4)];
    let j0 = |x: c64, h: f64| bessel_j(0, x * h);
    // ξ/(h J₁(ξh)) → 2/h² as ξ → 0
    let w = |x: c64, h: f64| x / (h * bessel_j(1, x * h));
    let (dx1, dx3, dy1, dy3) = (w(xi[0], hx), w(xi[2], hx), w(xi[0], hy), w(xi[2], hy));
    let dd = [w(xi[0], r), w(xi[1], r), w(xi[2], r), w(xi[3], r)];
    // (40), (41): ξ₁ towards +x and +y, ξ₃ towards −x and −y
    let lap = |k: Component| -> Vec<Term> {
        vec![
            (k, 1, 0, dx1),
            (k, -1, 0, dx3),
            (k, 0, 1, dy1),
            (k, 0, -1, dy3),
            (
                k,
                0,
                0,
                -(dx1 * j0(xi[0], hx)
                    + dx3 * j0(xi[2], hx)
                    + dy1 * j0(xi[0], hy)
                    + dy3 * j0(xi[2], hy)),
            ),
        ]
    };
    // the diagonal part of (42) and (43), each quadrant with its own ξ
    let diagonal = |k: Component| -> Vec<Term> {
        let mut t = vec![
            (k, 1, 1, dd[0]),
            (k, -1, 1, dd[1]),
            (k, -1, -1, dd[2]),
            (k, 1, -1, dd[3]),
        ];
        let centre: c64 = (0..4).map(|q| dd[q] * j0(xi[q], r)).sum();
        t.push((k, 0, 0, -centre));
        t
    };
    // (42)
    let mut diag_y = diagonal(Y);
    diag_y.extend([
        (Y, 0, 1, 2.0 * c2 * dy1),
        (Y, 0, -1, 2.0 * c2 * dy3),
        (
            Y,
            0,
            0,
            -2.0 * c2 * (dy1 * j0(xi[0], hy) + dy3 * j0(xi[2], hy)),
        ),
    ]);
    // (43)
    let mut diag_x = diagonal(X);
    diag_x.extend([
        (X, 1, 0, -2.0 * c2 * dx1),
        (X, -1, 0, -2.0 * c2 * dx3),
        (
            X,
            0,
            0,
            2.0 * c2 * (dx1 * j0(xi[0], hx) + dx3 * j0(xi[2], hx)),
        ),
    ]);
    // (44): ξ₁/J₁(ξ₁Δx) = Δx dx1, and so on
    let ez = vec![
        (Y, 1, 0, hx * dx1),
        (Y, 0, 0, -hx * dx1 * j0(xi[0], hx)),
        (Y, -1, 0, -hx * dx3),
        (Y, 0, 0, hx * dx3 * j0(xi[2], hx)),
        (X, 0, 1, -hy * dy1),
        (X, 0, 0, hy * dy1 * j0(xi[0], hy)),
        (X, 0, -1, hy * dy3),
        (X, 0, 0, -hy * dy3 * j0(xi[2], hy)),
    ];
    let k2 = g.k2;
    // α ≡ ε₂ + ε₄ − ε₁ − ε₃ (after Eq. 49), and ξ₂² − ξ₄²
    let alpha = e2 + e4 - e1 - e3;
    let dxi = k2 * (e2 - e4);
    let ln = (hx / hy).ln();
    // θ sin 2θ as in Eq. (47); Eqs. (50) and (52) misprint it as θ sin θ (see the module docs)
    let sin_term = th * s2 + c2 * th.sin().ln();
    let cos_term = (PI / 2.0 - th) * s2 - c2 * th.cos().ln();
    let ka = k2 * alpha;
    let scaled = |terms: Vec<Term>, f: f64| -> Vec<Term> {
        terms
            .into_iter()
            .map(|(k, i, j, v)| (k, i, j, v * f))
            .collect()
    };
    let mut out = lap(c);
    let (f1, f2, f3, hx_centre, hy_centre) = match c {
        // (50)
        Y => (
            ca1,
            ca2,
            ca3,
            -ka / PI * ln - 0.5 * ca1 * hy * dxi - 0.5 * ka * c2 * ca3
                + 2.0 * ka * ca2 / PI * sin_term,
            0.5 * ka - 0.5 * ca1 * hx * dxi + 0.5 * ka * c2 * ca2 + 2.0 * ka * ca3 / PI * cos_term,
        ),
        // (52)
        X => (
            cb1,
            cb2,
            cb3,
            0.5 * ka - 0.5 * cb1 * hy * dxi - 0.5 * ka * c2 * cb3 + 2.0 * ka * cb2 / PI * sin_term,
            ka / PI * ln - 0.5 * cb1 * hx * dxi
                + 0.5 * ka * c2 * cb2
                + 2.0 * ka * cb3 / PI * cos_term,
        ),
    };
    out.extend(scaled(ez, f1));
    out.extend(scaled(diag_y, f2));
    out.extend(scaled(diag_x, f3));
    out.push((X, 0, 0, c64::new(hx_centre, 0.0)));
    out.push((Y, 0, 0, c64::new(hy_centre, 0.0)));
    Ok(out)
}

/// The permittivities around node (i, j): north-east, north-west, south-west, south-east
/// (Hadley II's ε₁ to ε₄). Beyond the window's edges the edge cells continue (the mirror image
/// behind a wall).
fn quadrants(cs: &CrossSection, i: usize, j: usize) -> [f64; 4] {
    let (i, j) = (i as isize, j as isize);
    let e = |a: isize, b: isize| cs.cell(a, b).xx.re;
    [e(i, j), e(i - 1, j), e(i - 1, j - 1), e(i, j - 1)]
}

/// The equation of component `c` at a node with permittivities `q` around it.
fn equation(q: [f64; 4], c: Component, g: &Grid) -> Result<Vec<Term>> {
    let [e1, e2, e3, e4] = q;
    let place = |(same, other): (Vec<Local>, Vec<Local>), swap: bool| -> Vec<Term> {
        let at = |(a, b, v): Local, k: Component| {
            if swap { (k, b, a, v) } else { (k, a, b, v) }
        };
        let mut out: Vec<Term> = same.into_iter().map(|t| at(t, c)).collect();
        out.extend(other.into_iter().map(|t| at(t, c.other())));
        out
    };
    if e1 == e2 && e2 == e3 && e3 == e4 {
        uniform(e1, c, g)
    } else if e1 == e2 && e3 == e4 {
        // horizontal: region 1 above; H_y is normal to it
        let eq = interface(e1, e3, g.hx, g.hy, c == Component::Y, g)?;
        Ok(place(eq, false))
    } else if e1 == e4 && e2 == e3 {
        // vertical: x and y exchanged, region 1 to the east; H_x is normal to it
        let eq = interface(e1, e2, g.hy, g.hx, c == Component::X, g)?;
        Ok(place(eq, true))
    } else {
        corner(q, c, g)
    }
}

/// The matrix M(ε̄) of every node's equation, as (row, column, value) entries over the
/// unknowns, in an order that doesn't depend on ε̄.
fn assemble(
    cs: &CrossSection,
    numbering: &Numbering,
    g: &Grid,
) -> Result<Vec<(usize, usize, c64)>> {
    let (nxn, nyn) = (cs.x().len(), cs.y().len());
    let mut cache: HashMap<([u64; 4], Component), Vec<Term>> = HashMap::new();
    let mut entries = Vec::with_capacity(2 * nxn * nyn * 12);
    for i in 0..nxn {
        for j in 0..nyn {
            let q = quadrants(cs, i, j);
            for c in [Component::X, Component::Y] {
                let Some(row) = numbering.row(c, i, j) else {
                    continue;
                };
                let key = (q.map(f64::to_bits), c);
                if let std::collections::hash_map::Entry::Vacant(e) = cache.entry(key) {
                    e.insert(equation(q, c, g)?);
                }
                for &(k, di, dj, v) in &cache[&key] {
                    if let Some((col, sign)) =
                        numbering.neighbour(k, i as isize + di, j as isize + dj)
                    {
                        entries.push((row, col, v * sign));
                    }
                }
            }
        }
    }
    Ok(entries)
}

fn sparse(n: usize, entries: &[(usize, usize, c64)]) -> Result<SparseColMat<usize, c64>> {
    let triplets: Vec<Triplet<usize, usize, c64>> = entries
        .iter()
        .map(|&(i, j, v)| Triplet::new(i, j, v))
        .collect();
    SparseColMat::try_new_from_triplets(n, n, &triplets)
        .map_err(|e| Error::invalid("Hadley's equations", format!("{e:?}")))
}

fn dot(u: &[c64], v: &[c64]) -> c64 {
    u.iter().zip(v).map(|(a, b)| a.conj() * b).sum()
}

fn norm(v: &[c64]) -> f64 {
    v.iter().map(|x| x.norm_sqr()).sum::<f64>().sqrt()
}

/// Nonlinear inverse iteration (Güttel and Tisseur, Algorithm 4.7) on M(ε̄) h = 0 from
/// (`eb`, `start`): solve M(ε̄ₖ) w = M′(ε̄ₖ) vₖ, set ε̄ₖ₊₁ = ε̄ₖ − (u·vₖ)/(u·w) with u the
/// starting vector, and vₖ₊₁ = w/‖w‖. M′ by central differences. Returns ε̄ and h.
fn refine(
    cs: &CrossSection,
    numbering: &Numbering,
    k2: f64,
    (hx, hy): (f64, f64),
    eb: c64,
    start: Vec<c64>,
) -> Result<(c64, Vec<c64>)> {
    let n = numbering.unknowns.len();
    let s = norm(&start);
    let u: Vec<c64> = start.iter().map(|x| x / s).collect();
    let mut v = u.clone();
    let mut eb = eb;
    for _ in 0..30 {
        let grid = |eb: c64| Grid { k2, eb, hx, hy };
        let m = assemble(cs, numbering, &grid(eb))?;
        let delta = 1e-6 * eb.norm().max(1.0);
        let plus = assemble(cs, numbering, &grid(eb + delta))?;
        let minus = assemble(cs, numbering, &grid(eb - delta))?;
        // M′ v
        let mut rhs = vec![ZERO; n];
        for ((p, q), &(i, j, _)) in plus.iter().zip(&minus).zip(&m) {
            rhs[i] += (p.2 - q.2) / (2.0 * delta) * v[j];
        }
        let lu = sparse(n, &m)?
            .sp_lu()
            .map_err(|e| Error::invalid("Hadley's equations", format!("singular system: {e:?}")))?;
        let mut w = Mat::<c64>::from_fn(n, 1, |i, _| rhs[i]);
        lu.solve_in_place(w.as_mut());
        let w: Vec<c64> = (0..n).map(|i| w[(i, 0)]).collect();
        let step = dot(&u, &v) / dot(&u, &w);
        if !(step.re.is_finite() && step.im.is_finite()) {
            break;
        }
        eb -= step;
        let wn = norm(&w);
        v = w.iter().map(|x| x / wn).collect();
        if step.norm() <= 1e-13 * eb.norm() {
            return Ok((eb, v));
        }
    }
    Err(Error::invalid(
        "Hadley's equations",
        format!("the nonlinear inverse iteration didn't converge (last n_eff^2 {eb})"),
    ))
}

/// The uniform spacing of `v`, if it has one (to 1e-9 relative).
fn spacing(v: &[f64]) -> Option<f64> {
    let h = (v[v.len() - 1] - v[0]) / (v.len() - 1) as f64;
    v.windows(2)
        .all(|w| ((w[1] - w[0]) - h).abs() <= 1e-9 * h)
        .then_some(h)
}

/// Checks that `cs` is a problem Hadley's equations cover, and returns its spacings.
fn check(cs: &CrossSection) -> Result<(f64, f64)> {
    let (Some(hx), Some(hy)) = (spacing(cs.x()), spacing(cs.y())) else {
        return Err(Error::invalid(
            "Hadley's equations",
            "need a uniform grid along each axis (the spacings along x and y may differ)",
        ));
    };
    let p = cs.pml();
    if [p.west, p.east, p.south, p.north].iter().any(|&t| t > 0.0) {
        return Err(Error::invalid(
            "Hadley's equations",
            "don't take a PML: they are derived for real media",
        ));
    }
    let (nx, ny) = (cs.x().len() - 1, cs.y().len() - 1);
    for i in 0..nx {
        for j in 0..ny {
            let e = cs.cell(i as isize, j as isize);
            let real_iso = e.xx.im == 0.0
                && e.xx == e.yy
                && e.xx == e.zz
                && e.xy.norm() == 0.0
                && e.yx.norm() == 0.0
                && e.xx.re > 0.0;
            if !real_iso {
                return Err(Error::invalid(
                    "Hadley's equations",
                    "need isotropic, lossless media (real positive permittivity in every cell)",
                ));
            }
        }
    }
    Ok((hx, hy))
}

/// The `count` modes of `cs` at `wavelength` with effective indices nearest `near` (the
/// highest index in the cross-section when `None`), by Hadley's high-accuracy equations.
///
/// Each mode starts from the one [`vector::modes`] finds on the same grid and is refined until
/// ε̄ = n_eff² changes by less than 1e-13 relative. The cross-section's grid must be uniform
/// along each axis, its media isotropic and lossless, without a PML; mirror walls and zero
/// boundaries work as in [`vector`].
///
/// # Errors
///
/// [`Error::InvalidValue`] for a cross-section outside that scope, a grid so coarse that a
/// stencil's Bessel arguments |ξ|r exceed 12, a corner with ε₁ε₃ = ε₂ε₄ (Hadley's corner
/// equation is undefined there), or an iteration that fails to converge; and the errors of
/// [`vector::modes`].
pub fn modes(
    cs: &CrossSection,
    wavelength: Wavelength,
    count: usize,
    near: Option<f64>,
) -> Result<Vec<VectorMode>> {
    let (hx, hy) = check(cs)?;
    let start = vector::modes(cs, wavelength, count, near)?;
    let k = wavelength.wavenumber();
    let k2 = k * k;
    let numbering = Numbering::new(cs);
    let r = hx.hypot(hy);
    start
        .iter()
        .map(|mode| {
            let eb = mode.effective_index().powi(2);
            // every medium's |ξ| r, at the starting ε̄
            let worst = (0..cs.x().len() - 1)
                .flat_map(|i| (0..cs.y().len() - 1).map(move |j| (i, j)))
                .map(|(i, j)| {
                    (k2 * (cs.cell(i as isize, j as isize).xx - eb))
                        .norm()
                        .sqrt()
                        * r
                })
                .fold(0.0, f64::max);
            if worst > MAX_ARGUMENT {
                return Err(Error::invalid(
                    "Hadley's equations",
                    format!(
                        "the grid is too coarse: a stencil's Bessel argument |xi| r is {worst:.1}, \
                         above {MAX_ARGUMENT}"
                    ),
                ));
            }
            let v0 = mode.to_unknowns(&numbering.unknowns);
            let (eb, v) = refine(cs, &numbering, k2, (hx, hy), c64::new(eb.re, 0.0), v0)?;
            Ok(VectorMode::from_unknowns(
                cs,
                k,
                eb * k2,
                &numbering.unknowns,
                &v,
            ))
        })
        .collect()
}

/// Hadley I's uniform test problem (his Fig. 5): n = 3.44 at λ = 1.15 µm in a 2 × 2 µm box,
/// on `n` × `n` cells, its field H_y = cos(πx/4) sin(πy/2): zero derivative on the west edge (a
/// plane of symmetry) and zero on the others. Returns the cross-section, the wavelength and the
/// exact effective index, √(ε − ((π/4)² + (π/2)²)/k²).
pub(crate) fn uniform_box(n: usize) -> (CrossSection, Wavelength, f64) {
    use vector::Boundary::{ElectricWall, MagneticWall};
    let eps = 3.44f64 * 3.44;
    let cs = CrossSection::uniform((0.0, 2.0, n), (0.0, 2.0, n), |_, _| {
        vector::Permittivity::isotropic(c64::new(eps, 0.0))
    })
    .and_then(|cs| {
        // H_y is tangential to the west and east edges, normal to the others
        cs.with_boundaries(vector::Boundaries {
            west: ElectricWall,
            east: MagneticWall,
            south: ElectricWall,
            north: ElectricWall,
        })
    })
    .expect("a valid box");
    let w = Wavelength::from_um_unchecked(1.15);
    let k = w.wavenumber();
    let exact = (eps - ((PI / 4.0).powi(2) + (PI / 2.0).powi(2)) / (k * k)).sqrt();
    (cs, w, exact)
}

/// κ tan κL as a function of κ² (−γ tanh γL for κ² = −γ² < 0).
fn tan_term(kappa2: f64, l: f64) -> f64 {
    if kappa2 >= 0.0 {
        let k = kappa2.sqrt();
        k * (k * l).tan()
    } else {
        let g = (-kappa2).sqrt();
        -g * (g * l).tanh()
    }
}

/// Hadley I's interface test problem (his Fig. 6): a 1.5 µm wide box at λ = 0.975 µm, 1.5 µm
/// of ε = 11.8336 under 0.5 µm of ε = 1 (`high` contrast) or 10.89 under 10.5, with `n` cells
/// across the top layer (so 3n across the bottom one and the width), H_y of zero derivative at
/// the top and bottom edges and zero at the sides, H_x the reverse: all four edges magnetic
/// walls. `vertical` turns it on its side (x and y exchanged), so H_x is the component normal
/// to the interface. Separable: H_y = sin(πx/W) Y(y), with Y and Y′ continuous, so the exact
/// index solves κ_b tan κ_b L_b + κ_t tan κ_t L_t = 0, κ² = k²(ε − ε̄) − (π/W)². Returns the
/// cross-section, the wavelength and the exact effective index of the mode with H_y ≈
/// sin(πx/W) cos(κ_b y).
pub(crate) fn two_dielectric_box(
    high: bool,
    n: usize,
    vertical: bool,
) -> (CrossSection, Wavelength, f64) {
    use vector::Boundary::MagneticWall;
    let (top, bottom) = if high { (1.0, 11.8336) } else { (10.5, 10.89) };
    let (lt, lb, width) = (0.5, 1.5, 1.5);
    let eps = move |across: f64| if across > lb { top } else { bottom };
    let cs = if vertical {
        CrossSection::uniform((0.0, lt + lb, 4 * n), (0.0, width, 3 * n), |x, _| {
            vector::Permittivity::isotropic(c64::new(eps(x), 0.0))
        })
    } else {
        CrossSection::uniform((0.0, width, 3 * n), (0.0, lt + lb, 4 * n), |_, y| {
            vector::Permittivity::isotropic(c64::new(eps(y), 0.0))
        })
    }
    .and_then(|cs| {
        cs.with_boundaries(vector::Boundaries {
            west: MagneticWall,
            east: MagneticWall,
            south: MagneticWall,
            north: MagneticWall,
        })
    })
    .expect("a valid box");
    let w = Wavelength::from_um_unchecked(0.975);
    let k2 = w.wavenumber().powi(2);
    let q2 = (PI / width).powi(2);
    let f = |eb: f64| tan_term(k2 * (bottom - eb) - q2, lb) + tan_term(k2 * (top - eb) - q2, lt);
    // κ_b L_b from π/2 (f → +∞) down to 0 (f < 0): f decreases with ε̄
    let (mut lo, mut hi) = (
        bottom - (q2 + (PI / (2.0 * lb)).powi(2)) / k2 + 1e-12,
        bottom - q2 / k2,
    );
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if f(mid) > 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (cs, w, (0.5 * (lo + hi)).sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The power series of J_n(ξr) in r, to degree `d`.
    fn series(n: usize, xi: c64, d: usize) -> Vec<c64> {
        let mut s = vec![ZERO; d + 1];
        let mut k = 0;
        while n + 2 * k <= d {
            let mut v = if k % 2 == 0 { ONE } else { -ONE };
            for _ in 0..n + 2 * k {
                v *= xi / 2.0;
            }
            for m in 1..=k {
                v /= m as f64;
            }
            for m in 1..=n + k {
                v /= m as f64;
            }
            s[n + 2 * k] = v;
            k += 1;
        }
        s
    }

    const DEGREE: usize = 18;

    /// The coefficients uₙ (n of one parity) with Σ wₙ uₙ J_n(ξr) equal to the series `target`
    /// through degree [`DEGREE`]: triangular, since J_n starts at rⁿ.
    fn match_series(target: &[c64], xi: c64, weight: impl Fn(usize) -> f64, odd: bool) -> Vec<c64> {
        let mut u = vec![ZERO; DEGREE + 1];
        let mut rest = target.to_vec();
        for n in (usize::from(odd)..=DEGREE).step_by(2) {
            if weight(n) == 0.0 {
                continue;
            }
            let s = series(n, xi, DEGREE);
            u[n] = rest[n] / (weight(n) * s[n]);
            for p in n..=DEGREE {
                rest[p] -= weight(n) * u[n] * s[p];
            }
        }
        u
    }

    /// Σ wₙ uₙ J_n(ξr) over one parity class of n, as a series.
    fn class_sum(u: &[c64], xi: c64, weight: impl Fn(usize) -> f64, odd: bool) -> Vec<c64> {
        let mut s = vec![ZERO; DEGREE + 1];
        for n in (usize::from(odd)..=DEGREE).step_by(2) {
            for (p, v) in series(n, xi, DEGREE).into_iter().enumerate() {
                s[p] += weight(n) * u[n] * v;
            }
        }
        s
    }

    /// An exact local solution of the Helmholtz equation about a point of the line y = 0:
    /// Σ J_n(ξ₁r)(cₙ cos nθ + dₙ sin nθ) above it and Σ J_n(ξ₂r)(aₙ cos nθ + bₙ sin nθ) below
    /// (Hadley I, Eq. 10).
    struct Field {
        x1: c64,
        x2: c64,
        c: Vec<c64>,
        d: Vec<c64>,
        a: Vec<c64>,
        b: Vec<c64>,
    }

    impl Field {
        fn at(&self, x: f64, y: f64) -> c64 {
            let r = x.hypot(y);
            let t = y.atan2(x);
            let (xi, c, d) = if y >= 0.0 {
                (self.x1, &self.c, &self.d)
            } else {
                (self.x2, &self.a, &self.b)
            };
            (0..c.len())
                .map(|n| {
                    let nt = n as f64 * t;
                    bessel_j(n as u32, xi * r) * (c[n] * nt.cos() + d[n] * nt.sin())
                })
                .sum()
        }
    }

    /// Region 1's coefficients, fixed but arbitrary, up to order 8.
    fn coefficients(seed: f64) -> (Vec<c64>, Vec<c64>) {
        let c = (0..=DEGREE)
            .map(|n| {
                let v = if n <= 8 {
                    (seed + 1.3 * n as f64).sin()
                } else {
                    0.0
                };
                c64::new(v, 0.0)
            })
            .collect();
        let d = (0..=DEGREE)
            .map(|n| {
                let v = if (1..=8).contains(&n) {
                    (seed + 0.7 * n as f64).cos()
                } else {
                    0.0
                };
                c64::new(v, 0.0)
            })
            .collect();
        (c, d)
    }

    /// The component normal to the interface: H and ∂H/∂y continuous (Hadley I, the four
    /// relations before Eq. 11).
    fn normal_field(x1: c64, x2: c64, seed: f64) -> Field {
        let (c, d) = coefficients(seed);
        let (mut a, mut b) = (vec![ZERO; DEGREE + 1], vec![ZERO; DEGREE + 1]);
        for odd in [false, true] {
            let ua = match_series(&class_sum(&c, x1, |_| 1.0, odd), x2, |_| 1.0, odd);
            let n = |n: usize| n as f64;
            let ub = match_series(&class_sum(&d, x1, n, odd), x2, n, odd);
            for n in (usize::from(odd)..=DEGREE).step_by(2) {
                a[n] = ua[n];
                b[n] = ub[n];
            }
        }
        Field { x1, x2, c, d, a, b }
    }

    /// The tangential component, given the normal one: H continuous, and Hadley I's Eq. (25),
    /// (1/ε₁)∂H_x/∂y|₊ − (1/ε₂)∂H_x/∂y|₋ = (1/ε₁ − 1/ε₂)∂H_y/∂x, by parity classes (Eqs. 27–28).
    fn tangential_field(x1: c64, x2: c64, (e1, e2): (f64, f64), normal: &Field) -> Field {
        let (c, d) = coefficients(1.1);
        let (mut a, mut b) = (vec![ZERO; DEGREE + 1], vec![ZERO; DEGREE + 1]);
        for odd in [false, true] {
            let ua = match_series(&class_sum(&c, x1, |_| 1.0, odd), x2, |_| 1.0, odd);
            let mut tb = class_sum(&d, x1, |n| n as f64 / e1, odd);
            // r ∂r Σ c′ₙ J_n(ξ₁r) has the series p sₚ
            let along = class_sum(&normal.c, x1, |_| 1.0, odd);
            for (p, v) in along.into_iter().enumerate() {
                tb[p] -= (1.0 / e1 - 1.0 / e2) * p as f64 * v;
            }
            let ub = match_series(&tb, x2, |n| n as f64 / e2, odd);
            for n in (usize::from(odd)..=DEGREE).step_by(2) {
                a[n] = ua[n];
                b[n] = ub[n];
            }
        }
        Field { x1, x2, c, d, a, b }
    }

    /// A stencil's residual on exact fields at spacings (h, s·h): its terms on `f` and on
    /// `other`, relative to the field at the node.
    fn residual(same: &[Local], others: &[Local], f: &Field, other: &Field, h: (f64, f64)) -> f64 {
        let on = |terms: &[Local], field: &Field| -> c64 {
            terms
                .iter()
                .map(|&(a, b, v)| v * field.at(a as f64 * h.0, b as f64 * h.1))
                .sum()
        };
        (on(same, f) + on(others, other)).norm() / f.at(0.0, 0.0).norm()
    }

    /// The order p of a residual ∝ hᵖ, from h and h/2.
    fn order(at: impl Fn(f64) -> f64, h: f64) -> f64 {
        (at(h) / at(h / 2.0)).log2()
    }

    #[test]
    fn bessel_matches_known_values() {
        let close = |a: c64, b: c64| (a - b).norm() < 1e-14 * b.norm().max(1.0);
        let re = |v: f64| c64::new(v, 0.0);
        assert!(close(bessel_j(0, re(1.0)), re(0.765_197_686_557_966_6)));
        assert!(close(bessel_j(1, re(2.5)), re(0.497_094_102_464_274_1)));
        assert!(close(bessel_j(4, re(6.0)), re(0.357_641_594_780_961_5)));
        // J_n(ix) = iⁿ I_n(x): I₀(1) = 1.2660658777520084, I₁(2) = 1.5906368546373291
        assert!(close(
            bessel_j(0, c64::new(0.0, 1.0)),
            re(1.266_065_877_752_008_4)
        ));
        assert!(close(
            bessel_j(1, c64::new(0.0, 2.0)),
            c64::new(0.0, 1.590_636_854_637_329)
        ));
    }

    #[test]
    fn the_uniform_equation_is_exact_through_the_published_order() {
        // Hadley I after Eq. (9): the truncation error is r⁶ cos 8θ₀ on a square grid and
        // r⁴ cos 6θ₀ otherwise, after division by r²; so the residual goes as r⁸ and r⁶
        let (k2, eb, eps) = (16.0, c64::new(4.0, 0.0), 12.0);
        for (stretch, expected) in [(1.0, 8.0), (1.5, 6.0)] {
            let at = |h: f64| {
                let g = Grid {
                    k2,
                    eb,
                    hx: h,
                    hy: stretch * h,
                };
                let xi = g.xi(eps);
                let (c, d) = coefficients(0.4);
                let f = Field {
                    x1: xi,
                    x2: xi,
                    a: c.clone(),
                    b: d.clone(),
                    c,
                    d,
                };
                let same: Vec<Local> = uniform(eps, Component::Y, &g)
                    .unwrap()
                    .into_iter()
                    .map(|(_, i, j, v)| (i, j, v))
                    .collect();
                residual(&same, &[], &f, &f, (h, stretch * h))
            };
            let p = order(at, 0.05);
            assert!((p - expected).abs() < 0.2, "stretch {stretch}: order {p}");
        }
    }

    #[test]
    fn the_interface_equations_annihilate_the_local_solutions() {
        // high contrast, ξ₁ real and ξ₂ imaginary; then the reverse
        for (e1, e2, eb) in [(12.0, 1.5, 4.0), (2.0, 11.0, 5.0)] {
            let g = |h: f64| Grid {
                k2: 16.0,
                eb: c64::new(eb, 0.0),
                hx: h,
                hy: h,
            };
            let (x1, x2) = (g(1.0).xi(e1), g(1.0).xi(e2));
            let hy = normal_field(x1, x2, 0.3);
            let hx = tangential_field(x1, x2, (e1, e2), &hy);
            // Eq. (20), on the normal component alone
            let normal = |h: f64| {
                let (same, other) = interface(e1, e2, h, h, true, &g(h)).unwrap();
                assert!(other.is_empty());
                residual(&same, &[], &hy, &hy, (h, h))
            };
            // Eq. (43), on the tangential component and the normal one
            let tangential = |h: f64| {
                let (same, other) = interface(e1, e2, h, h, false, &g(h)).unwrap();
                residual(&same, &other, &hx, &hy, (h, h))
            };
            let (pn, pt) = (order(normal, 0.04), order(tangential, 0.04));
            println!(
                "eps {e1}/{e2}: normal order {pn:.2} ({:.1e}), tangential {pt:.2} ({:.1e})",
                normal(0.02),
                tangential(0.02)
            );
            assert!(pn > 4.5, "normal: order {pn}");
            assert!(pt > 3.5, "tangential: order {pt}");
        }
    }

    /// The relative error of the first mode near `exact`, by Hadley's equations.
    fn error(cs: &CrossSection, w: Wavelength, exact: f64) -> f64 {
        let n = modes(cs, w, 1, Some(exact)).unwrap()[0].effective_index();
        assert!(n.im.abs() < 1e-12, "{n}");
        (n.re - exact) / exact
    }

    #[test]
    fn a_uniform_box_converges_at_sixth_order() {
        // Hadley I, Fig. 5: slope 6.03
        let e: Vec<f64> = [4, 8, 16]
            .into_iter()
            .map(|n| {
                let (cs, w, exact) = uniform_box(n);
                error(&cs, w, exact)
            })
            .collect();
        for pair in e.windows(2) {
            let p = (pair[0] / pair[1]).abs().log2();
            assert!((p - 6.0).abs() < 0.2, "{e:?}: order {p}");
        }
        assert!(e[2].abs() < 3e-12, "{e:?}");
    }

    #[test]
    fn an_interface_converges_at_sixth_order() {
        // Hadley I, Fig. 7: an apparent sixth order in the modal index, the error larger at
        // high contrast
        for (high, grids) in [(false, [2, 4, 8]), (true, [4, 8, 16])] {
            let e: Vec<f64> = grids
                .into_iter()
                .map(|n| {
                    let (cs, w, exact) = two_dielectric_box(high, n, false);
                    error(&cs, w, exact)
                })
                .collect();
            for pair in e.windows(2) {
                let p = (pair[0] / pair[1]).abs().log2();
                assert!((p - 6.0).abs() < 0.3, "high {high}: {e:?}: order {p}");
            }
        }
    }

    #[test]
    fn a_vertical_interface_is_a_horizontal_one_turned() {
        let (flat, w, exact) = two_dielectric_box(true, 4, false);
        let (side, _, _) = two_dielectric_box(true, 4, true);
        let a = &modes(&flat, w, 1, Some(exact)).unwrap()[0];
        let b = &modes(&side, w, 1, Some(exact)).unwrap()[0];
        let (na, nb) = (a.effective_index().re, b.effective_index().re);
        assert!((na - nb).abs() < 1e-12, "{na} vs {nb}");
        // H_y normal to the flat interface, H_x to the turned one
        assert!(a.te_fraction() > 0.99 && b.te_fraction() < 0.01);
    }

    #[test]
    fn the_corner_exponents_solve_both_forms_of_eq_36() {
        // a box corner (ε₃ = 8, the rest 1), an impinged one, and three media
        for e in [
            [1.0, 1.0, 8.0, 1.0],
            [1.0, 2.25, 2.25, 2.25],
            [1.0, 2.0, 12.0, 3.5],
        ] {
            let (nu, mu) = corner_exponents(e).unwrap();
            let [e1, e2, e3, e4] = e;
            // Eq. (37)
            let s = (e1 * e3 - e2 * e4).powi(2) / ((e1 + e2) * (e2 + e3) * (e3 + e4) * (e4 + e1));
            for p in [nu, mu] {
                assert!(((PI * p / 2.0).sin().powi(2) - s).abs() < 1e-14, "{e:?}");
                assert!((5.0 / 3.0..=7.0 / 3.0).contains(&p), "{e:?}: {p}");
            }
            assert!((nu + mu - 4.0).abs() < 1e-14);
        }
        // ε₁ε₃ = ε₂ε₄: the exponents coincide at 2, and the corner equation is undefined
        let g = Grid {
            k2: 16.0,
            eb: c64::new(1.5, 0.0),
            hx: 0.1,
            hy: 0.1,
        };
        let degenerate = [1.0, 2.0, 4.0, 2.0];
        assert!(corner_exponents(degenerate).is_none());
        assert!(corner(degenerate, Component::Y, &g).is_err());
    }

    #[test]
    fn hadleys_corner_problems_converge_at_about_second_order() {
        // Hadley II, Figs. 8–11: about second order; relative errors (ours) at 16 and 32:
        // 3.2e-5, 7.1e-6 (box, 2.25); 5.1e-6, 1.3e-6 (box, 8)
        let w = Wavelength::from_um_unchecked(1.5);
        for problem in [1, 2] {
            let e: Vec<f64> = [16, 32]
                .into_iter()
                .map(|n| {
                    let (cs, exact) = vector::hadley_problem(problem, n);
                    error(&cs, w, exact)
                })
                .collect();
            let p = (e[0] / e[1]).abs().log2();
            assert!(
                (p - 2.0).abs() < 0.25,
                "problem {problem}: {e:?}: order {p}"
            );
        }
    }

    #[test]
    fn rejects_what_the_equations_dont_cover() {
        use vector::Permittivity;
        let w = Wavelength::from_um_unchecked(1.5);
        let eps = |e: f64| Permittivity::isotropic(c64::new(e, 0.0));
        let ok = CrossSection::uniform((0.0, 1.0, 8), (0.0, 1.0, 8), |x, _| {
            eps(if x < 0.5 { 4.0 } else { 1.0 })
        })
        .unwrap();
        assert!(modes(&ok, w, 1, None).is_ok());
        // a graded grid
        let x = vec![0.0, 0.1, 0.3, 0.6, 1.0];
        let graded = CrossSection::new(x.clone(), x, vec![eps(2.0); 16]).unwrap();
        assert!(modes(&graded, w, 1, None).is_err());
        // a lossy medium
        let lossy = CrossSection::uniform((0.0, 1.0, 8), (0.0, 1.0, 8), |_, _| {
            Permittivity::isotropic(c64::new(4.0, 0.01))
        })
        .unwrap();
        assert!(modes(&lossy, w, 1, None).is_err());
        // a PML
        let pml = ok
            .clone()
            .with_pml(vector::Pml {
                east: 0.25,
                strength: 3.0,
                ..vector::Pml::default()
            })
            .unwrap();
        assert!(modes(&pml, w, 1, None).is_err());
        // too coarse for the series: |ξ| r > 12
        let coarse = CrossSection::uniform((0.0, 6.0, 3), (0.0, 6.0, 3), |x, _| {
            eps(if x < 2.0 { 12.0 } else { 1.0 })
        })
        .unwrap();
        assert!(modes(&coarse, w, 1, None).is_err());
    }
}
