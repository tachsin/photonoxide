//! The modes of a slice between two walls, with or without a PML (the module's first point).

use std::f64::consts::PI;

use num_complex::Complex64 as c64;
use rayon::prelude::*;

use super::{Eme, Slice, Wall};
use crate::mode::Polarization;
use crate::{Error, Result};

/// A layer as the solver sees it: its permittivity n², where it starts and its real thickness
/// (µm, from the lower wall's PML, if any), and its stretch: d̃/d, 1 + iσ in a PML, else 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Layer {
    pub(crate) eps: c64,
    pub(crate) x0: f64,
    pub(crate) width: f64,
    pub(crate) stretch: c64,
}

impl Layer {
    /// The thickness along the complex coordinate.
    fn d(&self) -> c64 {
        self.stretch * self.width
    }
}

/// A mode of a slice, normalized to ⟨E, H⟩ = 1 (the module's second point).
#[derive(Clone, Debug, PartialEq)]
pub struct Mode {
    polarization: Polarization,
    k: f64,
    nu: c64,
    effective_index: c64,
    pub(crate) layers: Vec<Layer>,
    /// (U, P) at each boundary between layers, the walls included.
    states: Vec<(c64, c64)>,
    /// The factor that normalizes U.
    scale: c64,
}

/// sin z / z, its series near 0.
fn sinc(z: c64) -> c64 {
    if z.norm() < 1e-3 {
        let z2 = z * z;
        1.0 - z2 / 6.0 * (1.0 - z2 / 20.0)
    } else {
        z.sin() / z
    }
}

/// The weight w in P = (w/k) dU/dx: 1 for TE, 1/n² for TM.
fn weight(polarization: Polarization, eps: c64) -> c64 {
    match polarization {
        Polarization::Te => c64::new(1.0, 0.0),
        Polarization::Tm => 1.0 / eps,
    }
}

/// The layer's transfer of (U, P) from its lower side to its upper: [[cos Φ, sin Φ/γ],
/// [−γ sin Φ, cos Φ]], each entry a function of α² = n² − ν alone (ν = n_eff²).
fn transfer(polarization: Polarization, k: f64, layer: &Layer, nu: c64) -> [[c64; 2]; 2] {
    let a2 = layer.eps - nu;
    let d = layer.d();
    let phi = (k * k * a2 * d * d).sqrt();
    let w = weight(polarization, layer.eps);
    let (c, s) = (phi.cos(), sinc(phi));
    [[c, k * d / w * s], [-w * a2 * k * d * s, c]]
}

/// Whether a wall makes U vanish (E_y at an electric wall for TE, H_y at a magnetic one for
/// TM), rather than P.
fn u_vanishes(polarization: Polarization, wall: Wall) -> bool {
    matches!(
        (polarization, wall),
        (Polarization::Te, Wall::Electric) | (Polarization::Tm, Wall::Magnetic)
    )
}

/// The state (U, P) a wall starts from.
fn start(polarization: Polarization, wall: Wall) -> (c64, c64) {
    if u_vanishes(polarization, wall) {
        (c64::new(0.0, 0.0), c64::new(1.0, 0.0))
    } else {
        (c64::new(1.0, 0.0), c64::new(0.0, 0.0))
    }
}

/// The slice's layers with the PML's added inside each wall, at continuation `t` of the PML's
/// strength and the layers' loss (0: lossless and no PML; 1: as given).
fn layers(eme: &Eme, slice: &Slice, t: f64) -> Vec<Layer> {
    let mut out = Vec::new();
    let mut x = 0.0;
    let mut push = |n: c64, width: f64, stretch: c64, out: &mut Vec<Layer>| {
        let n = c64::new(n.re, t * n.im);
        out.push(Layer {
            eps: n * n,
            x0: x,
            width,
            stretch,
        });
        x += width;
    };
    let pml = eme
        .pml
        .map(|p| (p.thickness, c64::new(1.0, t * p.strength)));
    let lower = eme.pml.is_some_and(|p| p.lower);
    let first = slice.layers[0].0;
    let last = slice.layers[slice.layers.len() - 1].0;
    if let Some((d, s)) = pml.filter(|_| lower) {
        push(first, d, s, &mut out);
    }
    for &(n, d) in &slice.layers {
        push(n, d, c64::new(1.0, 0.0), &mut out);
    }
    if let Some((d, s)) = pml {
        push(last, d, s, &mut out);
    }
    out
}

/// Prüfer's angle θ at the upper wall for real ν, from the lower wall's θ (0 for U = 0, π/2 for
/// P = 0), U = r sin θ, P = r cos θ: θ crosses each multiple of π upward where U vanishes, so
/// θ/π counts U's zeros; it grows as ν falls (the Sturm–Liouville comparison theorem).
fn prufer(polarization: Polarization, k: f64, layers: &[Layer], lower: Wall, nu: f64) -> f64 {
    let (mut u, mut p): (f64, f64) = if u_vanishes(polarization, lower) {
        (0.0, 1.0)
    } else {
        (1.0, 0.0)
    };
    let mut zeros = 0.0;
    for layer in layers {
        let eps = layer.eps.re;
        let d = layer.width;
        let w = match polarization {
            Polarization::Te => 1.0,
            Polarization::Tm => 1.0 / eps,
        };
        let a2 = eps - nu;
        if a2 > 0.0 {
            let alpha = a2.sqrt();
            let g = w * alpha;
            let psi = u.atan2(p / g);
            let phi = k * alpha * d;
            zeros += ((psi + phi) / PI).floor() - (psi / PI).floor();
            let (c, s) = (phi.cos(), phi.sin());
            (u, p) = (u * c + p / g * s, -g * u * s + p * c);
        } else if a2 < 0.0 {
            let kappa = (-a2).sqrt();
            let g = w * kappa;
            let phi = k * kappa * d;
            // U = u cosh + (p/g) sinh vanishes once at most, where tanh = −u g/p
            if u * p < 0.0 && -u * g / p <= phi.tanh() {
                zeros += 1.0;
            }
            let (c, s) = (phi.cosh(), phi.sinh());
            (u, p) = (u * c + p / g * s, g * u * s + p * c);
        } else {
            if u * p < 0.0 && -u * w / (p * k) <= d {
                zeros += 1.0;
            }
            u += p * k * d / w;
        }
        let r = u.hypot(p);
        (u, p) = (u / r, p / r);
    }
    let mut end = u.atan2(p);
    if end < 0.0 {
        end += PI;
    }
    if end >= PI {
        end -= PI;
    }
    zeros * PI + end
}

/// How many eigenvalues ν lie above `nu`: the multiples of the upper wall's angle (0 for U = 0,
/// π/2 for P = 0) that θ has passed.
fn count(eme: &Eme, k: f64, layers: &[Layer], nu: f64) -> usize {
    let theta = prufer(eme.polarization, k, layers, eme.walls[0], nu);
    let n = if u_vanishes(eme.polarization, eme.walls[1]) {
        (theta / PI).floor()
    } else {
        ((theta - 0.5 * PI) / PI).floor() + 1.0
    };
    n.max(0.0) as usize
}

/// The first `m` eigenvalues ν = n_eff² of the lossless problem without a PML, largest first:
/// each by bisection on [`count`], to round-off.
fn real_eigenvalues(eme: &Eme, k: f64, layers: &[Layer], m: usize) -> Vec<f64> {
    let top = layers.iter().map(|l| l.eps.re).fold(f64::MIN, f64::max);
    let bottom = layers.iter().map(|l| l.eps.re).fold(f64::MAX, f64::min);
    let width: f64 = layers.iter().map(|l| l.width).sum();
    let mut low = bottom - ((m as f64 + 2.0) * PI / (k * width)).powi(2);
    while count(eme, k, layers, low) < m {
        low = bottom - 2.0 * (bottom - low);
    }
    let high = top + 1e-9 * top.abs().max(1.0);
    (0..m)
        .into_par_iter()
        .map(|j| {
            // count(lo) ≥ j + 1 > count(hi)
            let (mut lo, mut hi) = (low, high);
            for _ in 0..200 {
                let mid = 0.5 * (lo + hi);
                if mid <= lo || mid >= hi {
                    break;
                }
                if count(eme, k, layers, mid) > j {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            0.5 * (lo + hi)
        })
        .collect()
}

/// The states swept from each wall at ν, (U, P) at every boundary between layers, each with
/// its log-scale: the lower wall's from below and the upper's from above, rescaled layer by
/// layer so that a thick evanescent layer doesn't overflow.
fn sweeps(eme: &Eme, k: f64, layers: &[Layer], nu: c64) -> [Vec<(c64, c64, f64)>; 2] {
    let pol = eme.polarization;
    let n = layers.len();
    let unit = |u: c64, p: c64, s: f64| {
        let r = u.norm().max(p.norm());
        (u / r, p / r, s + r.ln())
    };
    let (u, p) = start(pol, eme.walls[0]);
    let mut lower = vec![(u, p, 0.0)];
    for layer in layers {
        let m = transfer(pol, k, layer, nu);
        let (u, p, s) = lower[lower.len() - 1];
        lower.push(unit(
            m[0][0] * u + m[0][1] * p,
            m[1][0] * u + m[1][1] * p,
            s,
        ));
    }
    let (u, p) = start(pol, eme.walls[1]);
    let mut upper = vec![(u, p, 0.0); n + 1];
    for j in (0..n).rev() {
        let m = transfer(pol, k, &layers[j], nu);
        let (u, p, s) = upper[j + 1];
        // the inverse of a matrix of determinant 1
        upper[j] = unit(m[1][1] * u - m[0][1] * p, -m[1][0] * u + m[0][0] * p, s);
    }
    [lower, upper]
}

/// Where the two sweeps meet best: the boundary where both have grown most (each relative to its
/// largest), so that neither has had to follow a decaying field against a growing one. A guided
/// mode meets at its core, a mode leaking into the PML at the PML.
fn meeting(eme: &Eme, k: f64, layers: &[Layer], nu: c64) -> usize {
    let [lower, upper] = sweeps(eme, k, layers, nu);
    let top = |v: &[(c64, c64, f64)]| v.iter().map(|s| s.2).fold(f64::MIN, f64::max);
    let (tl, tu) = (top(&lower), top(&upper));
    (0..=layers.len())
        .max_by(|&a, &b| {
            let both = |j: usize| (lower[j].2 - tl).min(upper[j].2 - tu);
            both(a).total_cmp(&both(b))
        })
        .unwrap_or(0)
}

/// The walls' condition for ν: the Wronskian U_l P_u − P_l U_u of the states swept from the two
/// walls, at boundary `at`: an entire function of ν, the same at every boundary up to rounding
/// (the transfer matrices have determinant 1), which vanishes where the two are one field: the
/// modes. As (its value over e^s, s). (A ratio such as U/P at a wall, free of the scale, is no
/// use: beyond a guided mode's cladding it is flat but within e^(−2γL) of the mode, where its
/// zero and pole meet.)
fn residual(eme: &Eme, k: f64, layers: &[Layer], nu: c64, at: usize) -> (c64, f64) {
    let pol = eme.polarization;
    let unit = |u: c64, p: c64, s: f64| {
        let r = u.norm().max(p.norm());
        (u / r, p / r, s + r.ln())
    };
    let (u, p) = start(pol, eme.walls[0]);
    let mut l = (u, p, 0.0);
    for layer in &layers[..at] {
        let m = transfer(pol, k, layer, nu);
        l = unit(
            m[0][0] * l.0 + m[0][1] * l.1,
            m[1][0] * l.0 + m[1][1] * l.1,
            l.2,
        );
    }
    let (u, p) = start(pol, eme.walls[1]);
    let mut r = (u, p, 0.0);
    for layer in layers[at..].iter().rev() {
        let m = transfer(pol, k, layer, nu);
        r = unit(
            m[1][1] * r.0 - m[0][1] * r.1,
            -m[1][0] * r.0 + m[0][0] * r.1,
            r.2,
        );
    }
    (l.0 * r.1 - l.1 * r.0, l.2 + r.2)
}

/// Newton's method on [`residual`] from `guess`, the sweeps meeting where they meet best at the
/// guess, its derivative by central differences; until the step is at round-off, or stops
/// falling below `noise` (relative).
fn newton(eme: &Eme, k: f64, layers: &[Layer], guess: c64, noise: f64) -> Option<c64> {
    let at = meeting(eme, k, layers, guess);
    let mut nu = guess;
    let mut last = f64::INFINITY;
    for _ in 0..60 {
        let h = 1e-6 * (1.0 + nu.norm());
        let (f, s) = residual(eme, k, layers, nu, at);
        let (fp, sp) = residual(eme, k, layers, nu + h, at);
        let (fm, sm) = residual(eme, k, layers, nu - h, at);
        // f(ν ± h) on f(ν)'s scale
        let up = fp * (sp - s).exp();
        let down = fm * (sm - s).exp();
        let step = f * 2.0 * h / (up - down);
        if !step.re.is_finite() || !step.im.is_finite() {
            return None;
        }
        nu -= step;
        let size = step.norm();
        let floor = 1.0 + nu.norm();
        if size <= 1e-14 * floor || (size <= noise * floor && size >= 0.5 * last) {
            return Some(nu);
        }
        last = size;
    }
    None
}

/// dν/dt at (t, ν) along `path`, by implicit differentiation of the walls' condition
/// f(ν, t) = 0: −(∂f/∂t)/(∂f/∂ν), each by central differences.
fn slope(eme: &Eme, k: f64, path: &dyn Fn(f64) -> Vec<Layer>, t: f64, nu: c64) -> c64 {
    // each value on f(ν)'s scale (f itself is near 0)
    let ratio = |a: (c64, f64), b: (c64, f64)| a.0 * (a.1 - b.1).exp();
    let h = 1e-6 * (1.0 + nu.norm());
    let here = path(t);
    let at = meeting(eme, k, &here, nu);
    let f = residual(eme, k, &here, nu, at);
    let dnu = (ratio(residual(eme, k, &here, nu + h, at), f)
        - ratio(residual(eme, k, &here, nu - h, at), f))
        / (2.0 * h);
    let dt = 1e-6;
    let up = residual(eme, k, &path(t + dt), nu, at);
    let down = residual(eme, k, &path(t - dt), nu, at);
    let dfdt = (ratio(up, f) - ratio(down, f)) / (2.0 * dt);
    -dfdt / dnu
}

/// One eigenvalue followed along `path` from t = 0 to 1: each step predicted along the root's
/// tangent and corrected by Newton's method, and halved where the correction isn't small beside
/// the step, so that it can't have jumped to another mode's root.
fn follow_one(eme: &Eme, k: f64, path: &dyn Fn(f64) -> Vec<Layer>, start: c64) -> Option<c64> {
    let mut nu = start;
    let (mut t, mut dt): (f64, f64) = (0.0, 1.0 / 64.0);
    while t < 1.0 {
        let next = (t + dt).min(1.0);
        let step = next - t;
        let predicted = nu + slope(eme, k, path, t, nu) * step;
        let moved = (predicted - nu).norm();
        // what Newton's method resolves near two modes' meeting, or for a PML's modes, whose
        // eigenvalues are ill-conditioned (the operator is far from normal)
        let floor = 2e-6 * (1.0 + nu.norm());
        let raw = newton(eme, k, &path(next), predicted, 1e-6);
        let corrected = raw.filter(|c| (c - predicted).norm() <= 0.2 * moved + floor);
        match corrected {
            Some(c) => {
                if (c - predicted).norm() <= 0.02 * moved + floor {
                    dt = (2.0 * dt).min(0.25);
                }
                nu = c;
                t = next;
            }
            None => {
                dt *= 0.5;
                if dt < 1e-7 {
                    return None;
                }
            }
        }
    }
    Some(nu)
}

/// The modes of a window filled with one medium, the upper outer layer's, with the slice's PML:
/// U goes as sin or cos of kαx̃ with kαW̃ a multiple of π (walls alike) or an odd multiple of
/// π/2 (walls unlike), W̃ the window's width along the complex coordinate.
fn uniform(eme: &Eme, k: f64, layers: &[Layer], m: usize) -> (c64, Vec<c64>) {
    let reference = layers[layers.len() - 1].eps;
    let width: c64 = layers.iter().map(Layer::d).sum();
    let (a, b) = (
        u_vanishes(eme.polarization, eme.walls[0]),
        u_vanishes(eme.polarization, eme.walls[1]),
    );
    let first = match (a, b) {
        (true, true) => 1.0,
        (false, false) => 0.0,
        _ => 0.5,
    };
    let nus = (0..m)
        .map(|j| {
            let alpha = (j as f64 + first) * PI / (k * width);
            reference - alpha * alpha
        })
        .collect();
    (reference, nus)
}

/// The modes with a PML or loss: the uniform window's ([`uniform`]), PML and all, followed as
/// each layer's permittivity goes from the uniform one to its own, ε(t) = ε_u + t(ε − ε_u). The
/// high modes, which the slice hardly moves, follow in a few steps; a guided mode rises out of
/// the window's. An error if a mode is lost or two end as one.
fn follow(eme: &Eme, k: f64, slice: &Slice) -> Result<Vec<c64>> {
    let failed = || {
        Error::invalid(
            "eme modes",
            "following the modes from a uniform window failed (fewer modes, or another PML)",
        )
    };
    let end = layers(eme, slice, 1.0);
    let (reference, start) = uniform(eme, k, &end, eme.modes);
    let path = |t: f64| -> Vec<Layer> {
        end.iter()
            .map(|l| Layer {
                eps: reference + t * (l.eps - reference),
                ..*l
            })
            .collect()
    };
    let nus: Vec<c64> = start
        .par_iter()
        .map(|&v| {
            // followed to 1e-6 (near two modes' meeting Newton's method slows and its noise
            // grows), then polished to round-off where it can be
            follow_one(eme, k, &path, v).map(|nu| newton(eme, k, &end, nu, 1e-12).unwrap_or(nu))
        })
        .collect::<Option<_>>()
        .ok_or_else(failed)?;
    for (i, a) in nus.iter().enumerate() {
        for b in &nus[..i] {
            if (a - b).norm() <= 1e-8 * (1.0 + a.norm()) {
                return Err(failed());
            }
        }
    }
    // highest Re n_eff² first
    let mut nus = nus;
    nus.sort_by(|a, b| b.re.total_cmp(&a.re));
    Ok(nus)
}

/// The slice's first `eme.modes` modes.
pub(super) fn solve(eme: &Eme, slice: &Slice) -> Result<Vec<Mode>> {
    let k = eme.wavelength.wavenumber();
    let lossy = slice.layers.iter().any(|&(n, _)| n.im != 0.0);
    let nus = if eme.pml.is_some() || lossy {
        follow(eme, k, slice)?
    } else {
        let lossless = layers(eme, slice, 0.0);
        real_eigenvalues(eme, k, &lossless, eme.modes)
            .into_iter()
            .map(|v| c64::new(v, 0.0))
            .collect()
    };
    let layers = layers(eme, slice, 1.0);
    Ok(nus
        .par_iter()
        .map(|&nu| Mode::new(eme, k, layers.clone(), nu))
        .collect())
}
impl Mode {
    /// The mode at ν: its states by sweeps from both walls, matched where both are large, and
    /// its normalization.
    fn new(eme: &Eme, k: f64, layers: Vec<Layer>, nu: c64) -> Mode {
        let pol = eme.polarization;
        let n = layers.len();
        let mut from_lower = vec![start(pol, eme.walls[0])];
        for layer in &layers {
            let m = transfer(pol, k, layer, nu);
            let (u, p) = from_lower[from_lower.len() - 1];
            from_lower.push((m[0][0] * u + m[0][1] * p, m[1][0] * u + m[1][1] * p));
        }
        let mut from_upper = vec![start(pol, eme.walls[1]); n + 1];
        for j in (0..n).rev() {
            let m = transfer(pol, k, &layers[j], nu);
            let (u, p) = from_upper[j + 1];
            // the inverse of a matrix of determinant 1
            from_upper[j] = (m[1][1] * u - m[0][1] * p, -m[1][0] * u + m[0][0] * p);
        }
        let size = |s: &(c64, c64)| s.0.norm().max(s.1.norm());
        let top_lower = from_lower.iter().map(size).fold(0.0, f64::max);
        let top_upper = from_upper.iter().map(size).fold(0.0, f64::max);
        let meet = (0..=n)
            .max_by(|&a, &b| {
                let both = |j: usize| {
                    (size(&from_lower[j]) / top_lower).min(size(&from_upper[j]) / top_upper)
                };
                both(a).total_cmp(&both(b))
            })
            .unwrap_or(0);
        let (l, r) = (from_lower[meet], from_upper[meet]);
        let ratio = (l.0 * r.0.conj() + l.1 * r.1.conj()) / (r.0.norm_sqr() + r.1.norm_sqr());
        let states: Vec<(c64, c64)> = (0..=n)
            .map(|j| {
                if j <= meet {
                    from_lower[j]
                } else {
                    (ratio * from_upper[j].0, ratio * from_upper[j].1)
                }
            })
            .collect();
        let mut effective_index = nu.sqrt();
        // the principal root, Re ≥ 0, unless that grows along z: an evanescent mode's +i√|ν|
        if effective_index.im < -1e-9 * effective_index.norm() {
            effective_index = -effective_index;
        }
        let mut mode = Mode {
            polarization: pol,
            k,
            nu,
            effective_index,
            layers,
            states,
            scale: c64::new(1.0, 0.0),
        };
        // ⟨E, H⟩ = n_eff ∫ w U² dx̃ with w = 1 (TE) or 1/n² (TM)
        let points = super::scattering::points(&[&mode], &[&mode]);
        let integral: c64 = points
            .iter()
            .map(|q| {
                let u = mode.u_at(q.layer_a, q.x);
                q.weight * u * u * weight(pol, mode.layers[q.layer_a].eps)
            })
            .sum();
        mode.scale = 1.0 / (effective_index * integral).sqrt();
        mode
    }

    /// TE or TM.
    pub fn polarization(&self) -> Polarization {
        self.polarization
    }

    /// The effective index, n_eff = β/k: Im n_eff > 0 for a lossy or evanescent mode.
    pub fn effective_index(&self) -> c64 {
        self.effective_index
    }

    /// The window's width with its PMLs, µm.
    pub fn width(&self) -> f64 {
        self.layers.iter().map(|l| l.width).sum()
    }

    /// The tangential fields at x (µm from the lower wall, PMLs included), normalized to
    /// ⟨E, H⟩ = 1 in units of Z₀ for E: (E_y, H_x) for TE, (E_x, H_y) for TM.
    pub fn field(&self, x: f64) -> (c64, c64) {
        let j = self.layers.iter().rposition(|l| l.x0 <= x).unwrap_or(0);
        let u = self.scale * self.u_at(j, x);
        match self.polarization {
            Polarization::Te => (u, -self.effective_index * u),
            Polarization::Tm => (self.effective_index * u / self.layers[j].eps, u),
        }
    }

    /// (e, h) at x in layer j such that ⟨E_a, H_b⟩ = ∫ e_a h_b dx̃: for TE e = U and
    /// h = n_eff U (−E_y H_x = n_eff E_y²); for TM e = n_eff U/n² and h = U (E_x H_y), both
    /// normalized.
    pub(crate) fn e_h(&self, j: usize, x: f64) -> (c64, c64) {
        let u = self.scale * self.u_at(j, x);
        match self.polarization {
            Polarization::Te => (u, self.effective_index * u),
            Polarization::Tm => (self.effective_index * u / self.layers[j].eps, u),
        }
    }

    /// The largest |kα| of the mode in layer j, which sets how finely it is integrated.
    pub(crate) fn wavenumber(&self, j: usize) -> f64 {
        self.k * (self.layers[j].eps - self.nu).norm().sqrt()
    }

    /// U, unnormalized, at x in layer j: from the layer's lower state where it can't grow much
    /// across the layer, else as a e^(ikαt) + b e^(ikα(d̃ − t)) with a from the lower state and
    /// b from the upper, each term at most its end's size.
    fn u_at(&self, j: usize, x: f64) -> c64 {
        let layer = &self.layers[j];
        let t = layer.stretch * (x - layer.x0).clamp(0.0, layer.width);
        let d = layer.d();
        let mut alpha = (layer.eps - self.nu).sqrt();
        if (alpha * d).im < 0.0 {
            alpha = -alpha;
        }
        let k = self.k;
        let w = weight(self.polarization, layer.eps);
        let (u0, p0) = self.states[j];
        if (k * alpha * d).im <= 2.0 {
            let phi = k * alpha * t;
            u0 * phi.cos() + p0 * k * t / w * sinc(phi)
        } else {
            let i = c64::new(0.0, 1.0);
            let (u1, p1) = self.states[j + 1];
            let g = i * w * alpha;
            let a = 0.5 * (u0 + p0 / g);
            let b = 0.5 * (u1 - p1 / g);
            a * (i * k * alpha * t).exp() + b * (i * k * alpha * (d - t)).exp()
        }
    }
}
