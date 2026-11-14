//! A mode's derivative along a change of its slice: a layer's permittivity, or where a boundary
//! between two layers lies.
//!
//! Every quantity the mode is built from is evaluated on dual numbers ([`crate::mode::dual`]),
//! so its derivative is exact. ν = n_eff² is a root of the Wronskian W(ν, p) of the sweeps from
//! the two walls at a boundary where both are large; along the change p it moves by
//! dν = −(∂W/∂p)/(∂W/∂ν) (the implicit function theorem), both partials from the same sweeps on
//! duals. The states (U, P) at the boundaries then move by the sweep from the lower wall's
//! derivative, ∂/∂p + dν ∂/∂ν, up to that boundary, and above it by the upper sweep's scaled to
//! meet it (the scale ρ = l/r, dρ the projection of dl − ρ dr on r). The field inside a layer
//! is the same expression as [`super::Mode`]'s on duals, x held while the layer's start and
//! thickness move. The normalization ⟨E, H⟩ = n_eff ∫ w U² dx̃ = 1 moves by its integrand's
//! derivative at fixed x, plus, where a boundary moves and the integrand jumps across it (TM's
//! w = 1/n²), the jump times the boundary's motion (Leibniz's rule).

use num_complex::Complex64 as c64;

use super::{Eme, Layer, Mode, start};
use crate::mode::Polarization;
use crate::mode::dual::Dual;
use crate::portable;

/// A change of a slice's layers as the solver has them (its PMLs included): each one's
/// permittivity (and the physical one with it), start and thickness, per unit of the parameter.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LayerChange {
    pub(crate) eps: Vec<c64>,
    pub(crate) x0: Vec<f64>,
    pub(crate) width: Vec<f64>,
}

impl LayerChange {
    /// No change of `n` layers.
    pub(crate) fn none(n: usize) -> LayerChange {
        LayerChange {
            eps: vec![c64::new(0.0, 0.0); n],
            x0: vec![0.0; n],
            width: vec![0.0; n],
        }
    }

    /// The boundaries that move: (the layer below, its position, how fast).
    pub(crate) fn moving(&self, layers: &[Layer]) -> Vec<(usize, f64, f64)> {
        (1..layers.len())
            .filter(|&j| self.x0[j] != 0.0)
            .map(|j| (j - 1, layers[j].x0, self.x0[j]))
            .collect()
    }
}

/// A mode's derivative along a [`LayerChange`].
#[derive(Clone, Debug)]
pub(crate) struct ModeTangent {
    change: LayerChange,
    nu: c64,
    effective_index: c64,
    states: Vec<(c64, c64)>,
    scale: c64,
}

impl ModeTangent {
    /// d n_eff.
    pub(crate) fn effective_index(&self) -> c64 {
        self.effective_index
    }

    /// The change of the slice's layers it is along.
    pub(crate) fn change(&self) -> &LayerChange {
        &self.change
    }
}

/// sin z / z on duals, its series near 0 as [`super::sinc`]'s.
fn sinc(z: Dual) -> Dual {
    if portable::complex::abs(z.v) < 1e-3 {
        let z2 = z * z;
        Dual::from(1.0) - z2 * Dual::from(1.0 / 6.0) * (Dual::from(1.0) - z2 * Dual::from(0.05))
    } else {
        z.sin() / z
    }
}

impl Mode {
    /// Layer j's permittivity, start, complex thickness and weight w as duals along `change`.
    fn layer_duals(&self, change: &LayerChange, j: usize) -> (Dual, Dual, Dual, Dual) {
        let l = &self.layers[j];
        let eps = Dual::new(l.eps, change.eps[j]);
        let phys = Dual::new(l.phys, change.eps[j]);
        let x0 = Dual::new(c64::new(l.x0, 0.0), c64::new(change.x0[j], 0.0));
        let d = Dual::new(l.stretch * l.width, l.stretch * change.width[j]);
        let w = match self.polarization {
            Polarization::Te => Dual::from(1.0),
            Polarization::Tm => Dual::from(1.0) / phys,
        };
        (eps, x0, d, w)
    }

    /// Layer j's transfer of (U, P), as [`super::transfer`], on duals.
    fn transfer_dual(&self, change: &LayerChange, j: usize, nu: Dual) -> [[Dual; 2]; 2] {
        let (eps, _, d, w) = self.layer_duals(change, j);
        let k = self.k;
        let a2 = eps - nu;
        let phi = (a2 * d * d * (k * k)).sqrt();
        let (c, s) = (phi.cos(), sinc(phi));
        [[c, d * k / w * s], [-(w * a2 * d * s * k), c]]
    }

    /// The sweeps from the lower wall and from the upper one, states at every boundary.
    #[allow(clippy::type_complexity)]
    fn sweeps_dual(
        &self,
        eme: &Eme,
        change: &LayerChange,
        nu: Dual,
    ) -> (Vec<(Dual, Dual)>, Vec<(Dual, Dual)>) {
        let pol = self.polarization;
        let n = self.layers.len();
        let s = start(pol, eme.walls[0]);
        let mut lower = vec![(Dual::from(s.0), Dual::from(s.1))];
        for j in 0..n {
            let m = self.transfer_dual(change, j, nu);
            let (u, p) = lower[j];
            lower.push((m[0][0] * u + m[0][1] * p, m[1][0] * u + m[1][1] * p));
        }
        let s = start(pol, eme.walls[1]);
        let mut upper = vec![(Dual::from(s.0), Dual::from(s.1)); n + 1];
        for j in (0..n).rev() {
            let m = self.transfer_dual(change, j, nu);
            let (u, p) = upper[j + 1];
            upper[j] = (m[1][1] * u - m[0][1] * p, -(m[1][0] * u) + m[0][0] * p);
        }
        (lower, upper)
    }

    /// The mode's derivative along `change` (the module's docs).
    pub(crate) fn tangent(&self, eme: &Eme, change: &LayerChange) -> ModeTangent {
        let n = self.layers.len();
        let still = LayerChange::none(n);
        // along ν alone, and along the change alone
        let (la, ra) = self.sweeps_dual(eme, &still, Dual::variable(self.nu));
        let (lb, rb) = self.sweeps_dual(eme, change, Dual::constant(self.nu));
        let size = |s: &(Dual, Dual)| portable::complex::abs(s.0.v).max(portable::complex::abs(s.1.v));
        let top_l = la.iter().map(size).fold(0.0, f64::max);
        let top_r = ra.iter().map(size).fold(0.0, f64::max);
        let meet = (0..=n)
            .max_by(|&a, &b| {
                let both = |j: usize| (size(&la[j]) / top_l).min(size(&ra[j]) / top_r);
                both(a).total_cmp(&both(b))
            })
            .unwrap_or(0);
        let wronskian = |l: (Dual, Dual), r: (Dual, Dual)| l.0 * r.1 - l.1 * r.0;
        let w_nu = wronskian(la[meet], ra[meet]).d;
        let w_p = wronskian(lb[meet], rb[meet]).d;
        let dnu = -w_p / w_nu;
        let along = |a: (Dual, Dual), b: (Dual, Dual)| (b.0.d + dnu * a.0.d, b.1.d + dnu * a.1.d);
        let dl: Vec<(c64, c64)> = la.iter().zip(&lb).map(|(&a, &b)| along(a, b)).collect();
        let dr: Vec<(c64, c64)> = ra.iter().zip(&rb).map(|(&a, &b)| along(a, b)).collect();
        // above the meeting the upper sweep scaled to meet the lower: ρ = l/r
        let (l, r) = ((la[meet].0.v, la[meet].1.v), (ra[meet].0.v, ra[meet].1.v));
        let rr = r.0.norm_sqr() + r.1.norm_sqr();
        let rho = (l.0 * r.0.conj() + l.1 * r.1.conj()) / rr;
        let (dlm, drm) = (dl[meet], dr[meet]);
        let drho = ((dlm.0 - rho * drm.0) * r.0.conj() + (dlm.1 - rho * drm.1) * r.1.conj()) / rr;
        let states: Vec<(c64, c64)> = (0..=n)
            .map(|j| {
                if j <= meet {
                    dl[j]
                } else {
                    (
                        drho * ra[j].0.v + rho * dr[j].0,
                        drho * ra[j].1.v + rho * dr[j].1,
                    )
                }
            })
            .collect();
        let mut t = ModeTangent {
            change: change.clone(),
            nu: dnu,
            effective_index: dnu / (2.0 * self.effective_index),
            states,
            scale: c64::new(0.0, 0.0),
        };
        // the normalization's integral n_eff ∫ w U² dx̃ (unnormalized U) and its derivative
        let points = super::super::scattering::points(&[self], &[self]);
        let (mut integral, mut d_integral) = (c64::new(0.0, 0.0), c64::new(0.0, 0.0));
        for q in &points {
            let u = self.u_dual(&t, q.layer_a, q.x_a);
            let (_, _, _, w) = self.layer_duals(change, q.layer_a);
            let f = u * u * w;
            integral += q.weight * f.v;
            d_integral += q.weight * f.d;
        }
        for (j, b, speed) in change.moving(&self.layers) {
            let below = self.u_dual(&t, j, b);
            let above = self.u_dual(&t, j + 1, b);
            let (_, _, _, wb) = self.layer_duals(change, j);
            let (_, _, _, wa) = self.layer_duals(change, j + 1);
            let jump = below.v * below.v * wb.v - above.v * above.v * wa.v;
            d_integral += self.layers[j].stretch * jump * speed;
        }
        let neff = Dual::new(self.effective_index, t.effective_index);
        let total = neff * Dual::new(integral, d_integral);
        t.scale = (Dual::from(1.0) / total.sqrt()).d;
        t
    }

    /// U, unnormalized, at x in layer j, on duals: [`super::Mode::u_at`]'s expression.
    fn u_dual(&self, t: &ModeTangent, j: usize, x: f64) -> Dual {
        let layer = &self.layers[j];
        let (eps, x0, d, w) = self.layer_duals(&t.change, j);
        let stretch = Dual::from(layer.stretch);
        let inside = x - layer.x0;
        // within the layer the distance moves with its start; clamped at its ends as u_at is
        let along = if inside <= 0.0 {
            Dual::from(0.0)
        } else if inside >= layer.width {
            Dual::new(c64::new(layer.width, 0.0), c64::new(t.change.width[j], 0.0))
        } else {
            Dual::from(c64::new(x, 0.0)) - x0
        };
        let tt = stretch * along;
        let nu = Dual::new(self.nu, t.nu);
        let mut alpha = (eps - nu).sqrt();
        if (alpha.v * d.v).im < 0.0 {
            alpha = -alpha;
        }
        let k = self.k;
        let (s0, s1) = (self.states[j], t.states[j]);
        let (u0, p0) = (Dual::new(s0.0, s1.0), Dual::new(s0.1, s1.1));
        if (k * alpha.v * d.v).im <= 2.0 {
            let phi = alpha * tt * k;
            u0 * phi.cos() + p0 * tt * k / w * sinc(phi)
        } else {
            let i = Dual::from(c64::new(0.0, 1.0));
            let (e0, e1) = (self.states[j + 1], t.states[j + 1]);
            let (u1, p1) = (Dual::new(e0.0, e1.0), Dual::new(e0.1, e1.1));
            let g = i * w * alpha;
            let a = (u0 + p0 / g) * 0.5;
            let b = (u1 - p1 / g) * 0.5;
            a * (i * alpha * tt * k).exp() + b * (i * alpha * (d - tt) * k).exp()
        }
    }

    /// The normalized (e, h) at x in layer j and their derivatives, as [`Mode::e_h`] gives them.
    pub(crate) fn e_h_dual(&self, t: &ModeTangent, j: usize, x: f64) -> (Dual, Dual) {
        let scale = Dual::new(self.scale, t.scale);
        let u = scale * self.u_dual(t, j, x);
        let neff = Dual::new(self.effective_index, t.effective_index);
        match self.polarization {
            Polarization::Te => (u, neff * u),
            Polarization::Tm => {
                let (_, _, _, w) = self.layer_duals(&t.change, j);
                (neff * u * w, u)
            }
        }
    }

    /// The tangent of a mode that doesn't change.
    pub(crate) fn still(&self) -> ModeTangent {
        let n = self.layers.len();
        ModeTangent {
            change: LayerChange::none(n),
            nu: c64::new(0.0, 0.0),
            effective_index: c64::new(0.0, 0.0),
            states: vec![(c64::new(0.0, 0.0), c64::new(0.0, 0.0)); n + 1],
            scale: c64::new(0.0, 0.0),
        }
    }
}
