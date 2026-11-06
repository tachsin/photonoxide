//! A bent section: the conformal map that makes it a straight one, its mapped stack of thin
//! layers, and the map between its coordinate and the joint's.
//!
//! A section bent at radius R in the plane of the expansion (x across it, z along it) has its
//! layers on circles of radius ρ = R + ξ, ξ = x − x_c the distance from the reference radius at
//! the window's middle x_c. The map u = R ln(ρ/R), v = Rφ is conformal, and turns the 2D wave
//! equation into the straight guide's with the index n(u) e^(u/R): M. Heiblum, J. H. Harris,
//! IEEE J. Quantum Electron. 11, 75 (1975),
//! [doi:10.1109/JQE.1975.1068563](https://doi.org/10.1109/JQE.1975.1068563). For TE (E along y)
//! ∂²_u E + ∂²_v E + k²n²e^(2u/R) E = 0 exactly; for TM (H along y) the profile enters through
//! 1/n², and the map leaves it there: ∂_u(n⁻² ∂_u H) + ∂_v(n⁻² ∂_v H) + k²e^(2u/R) H = 0, so a
//! layer's weight in P = (w/k) dU/du stays 1/n² while its α² = n²e^(2u/R) − n_eff².
//!
//! The mapped profile is graded: the section's modes are those of a stack of thin layers, each
//! of the map's index at its middle in u (a staircase whose error falls as the square of the
//! layers' thickness). A mode goes as e^(iβv) = e^(iβRφ): its n_eff = β/k is the propagation
//! constant along the arc at the reference radius, ν = n_eff k R the angular order of
//! [`crate::mode::bend::SlabBend`].
//!
//! A PML keeps its physical thickness. In the mapped coordinate it is linear, its thickness the
//! physical one over dx/du at the window's edge, and the profile goes on into it as its analytic
//! continuation, n² e^(2ũ/R) along the complex coordinate ũ: the light the bend radiates leaves
//! through it without meeting a change in the profile's slope.
//!
//! At a joint, both sections' tangential fields are functions of the physical x along the joint's
//! plane. A bent mode's tangential E and H there are its mapped ones, but for the one that came
//! from a derivative along the arc (TE's H_x, TM's E_x), which is (R/ρ) times the mapped one:
//! (1/ρ)∂_φ = (R/ρ)∂_v. With dx = (ρ/R) du the two sections' product ∫ E × H dx is the mapped
//! one ∫ E × H du for two sections of the same bend, so that a bend's modes stay orthonormal and
//! two bends of one radius in sequence are one.

use num_complex::Complex64 as c64;

use super::modes::Layer;
use super::{Eme, Slice};
use crate::{Error, Result};

/// The map between a bent section's mapped coordinate s (along the real axis, from its lower
/// wall, PML included) and the physical x across the joint's plane (from the lower wall, PML
/// included).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Map {
    /// R, µm: positive with the centre of the bend below the lower wall (the upper wall is
    /// outside), negative with it above the upper one.
    radius: f64,
    /// The lower PML's physical thickness, µm (0 without one).
    lower: f64,
    /// The window's physical width between the PMLs, µm.
    width: f64,
    /// u at the window's lower and upper edges.
    u: [f64; 2],
    /// dx/du there: ρ/R.
    jacobian: [f64; 2],
}

impl Map {
    /// The map of a window `width` wide bent at `radius` to its middle, its lower PML `lower`
    /// thick.
    pub(crate) fn new(radius: f64, lower: f64, width: f64) -> Map {
        let half = 0.5 * width;
        let u = |xi: f64| radius * (xi / radius).ln_1p();
        Map {
            radius,
            lower,
            width,
            u: [u(-half), u(half)],
            jacobian: [1.0 - half / radius, 1.0 + half / radius],
        }
    }

    /// The lower PML's thickness in s.
    fn lower_s(&self) -> f64 {
        self.lower / self.jacobian[0]
    }

    /// s at physical x.
    pub(crate) fn to_s(&self, x: f64) -> f64 {
        let (lo, hi) = (self.lower, self.lower + self.width);
        let window = self.u[1] - self.u[0];
        if x < lo {
            x / self.jacobian[0]
        } else if x <= hi {
            let xi = x - lo - 0.5 * self.width;
            self.lower_s() + self.radius * (xi / self.radius).ln_1p() - self.u[0]
        } else {
            self.lower_s() + window + (x - hi) / self.jacobian[1]
        }
    }

    /// Physical x at s.
    pub(crate) fn to_x(&self, s: f64) -> f64 {
        let (lo, hi) = (self.lower_s(), self.lower_s() + self.u[1] - self.u[0]);
        if s < lo {
            s * self.jacobian[0]
        } else if s <= hi {
            let u = s - lo + self.u[0];
            self.lower + 0.5 * self.width + self.radius * (u / self.radius).exp_m1()
        } else {
            self.lower + self.width + (s - hi) * self.jacobian[1]
        }
    }

    /// dx/ds at physical x: ρ/R in the window, its value at the edge in a PML.
    pub(crate) fn jacobian(&self, x: f64) -> f64 {
        let (lo, hi) = (self.lower, self.lower + self.width);
        if x < lo {
            self.jacobian[0]
        } else if x <= hi {
            1.0 + (x - lo - 0.5 * self.width) / self.radius
        } else {
            self.jacobian[1]
        }
    }
}

/// The bend's window must stay on one side of its centre, PML and all.
pub(crate) fn check(eme: &Eme, slice: &Slice, radius: f64) -> Result<()> {
    let half = 0.5 * slice.width().to_um();
    let inner = if radius > 0.0 {
        half + eme.pml.filter(|p| p.lower).map_or(0.0, |p| p.thickness)
    } else {
        half + eme.pml.map_or(0.0, |p| p.thickness)
    };
    if inner >= radius.abs() {
        return Err(Error::invalid(
            "eme bend",
            format!(
                "the window reaches {inner} um towards the bend's centre, at least its radius \
                 ({} um): a narrower window on the inside, or a larger radius",
                radius.abs()
            ),
        ));
    }
    Ok(())
}

/// The map of `slice` bent at `radius` under `eme`'s PML.
pub(crate) fn map(eme: &Eme, slice: &Slice, radius: f64) -> Map {
    let lower = eme.pml.filter(|p| p.lower).map_or(0.0, |p| p.thickness);
    Map::new(radius, lower, slice.width().to_um())
}

/// The bent slice's mapped stack of thin layers, at continuation `t` of the PML's strength and
/// the layers' loss (as the straight slice's): each original layer, and each PML, cut into
/// layers at most `eme`'s step thick in s, each of the map's permittivity at its middle,
/// n² e^(2ũ/R), with ũ complex in a PML.
pub(crate) fn layers(eme: &Eme, slice: &Slice, radius: f64, t: f64) -> Vec<Layer> {
    let map = map(eme, slice, radius);
    let step = eme.bend_step;
    let mut out = Vec::new();
    let mut s = 0.0;
    // `count` layers of index n across `width` of s, the mapped ũ at s given by `u_at`
    let mut push = |n: c64, width: f64, stretch: c64, u_at: &dyn Fn(f64) -> c64| {
        let n = c64::new(n.re, t * n.im);
        // a power of two, so that halving the step doubles every count: the staircases of
        // successive steps are nested, and their errors fall as the step's square exactly
        let count = ((width / step).ceil().max(1.0) as usize).next_power_of_two();
        let h = width / count as f64;
        for j in 0..count {
            let mid = s + (j as f64 + 0.5) * h;
            let bend = 2.0 * u_at(mid) / radius;
            out.push(Layer {
                eps: n * n * bend.exp(),
                phys: n * n,
                bend,
                x0: s + j as f64 * h,
                width: h,
                stretch,
            });
        }
        s += width;
    };
    let pml = eme.pml.map(|p| (p.thickness, c64::new(1.0, t * p.strength)));
    let lower = eme.pml.is_some_and(|p| p.lower);
    let first = slice.layers[0].0;
    let last = slice.layers[slice.layers.len() - 1].0;
    let s_lo = map.lower_s();
    let [u_lo, u_hi] = map.u;
    if let Some((_, st)) = pml.filter(|_| lower) {
        push(first, s_lo, st, &move |s: f64| u_lo + st * (s - s_lo));
    }
    let mut xi = -0.5 * map.width;
    for &(n, d) in &slice.layers {
        let u = |xi: f64| radius * (xi / radius).ln_1p();
        let (a, b) = (u(xi), u(xi + d));
        let base = s_lo - u_lo;
        push(n, b - a, c64::new(1.0, 0.0), &move |s: f64| c64::new(s - base, 0.0));
        xi += d;
    }
    if let Some((d, st)) = pml {
        let s_hi = s_lo + u_hi - u_lo;
        push(last, d / map.jacobian[1], st, &move |s: f64| u_hi + st * (s - s_hi));
    }
    out
}
