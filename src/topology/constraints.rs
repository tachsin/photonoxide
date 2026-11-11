//! Minimum linewidth and linespacing as constraints an optimizer keeps: Zhou et al.'s geometric
//! constraints, in the foundry form Hammond et al. give them.
//!
//! M. Zhou, B. S. Lazarov, F. Wang, O. Sigmund, "Minimum length scale in topology optimization by
//! geometric constraints", Comput. Methods Appl. Mech. Engrg. 293, 266 (2015),
//! doi:10.1016/j.cma.2015.05.003, Section 2: the filtered field ρ̃ must reach the eroded
//! threshold η_e where it is flat inside the solid and stay below the dilated threshold η_d where
//! it is flat inside the void; indicators I_s = ρ̄ exp(−c|∇ρ̃|²) and I_v = (1 − ρ̄) exp(−c|∇ρ̃|²)
//! (their Eqs. 8 and 9) find those places, and
//!
//! g_s = (1/n) Σ I_s min(ρ̃ − η_e, 0)², g_v = (1/n) Σ I_v min(η_d − ρ̃, 0)²
//!
//! (Eqs. 10 and 11) are held to g ≤ ε. ∇ρ̃ is taken per pixel by central differences (one-sided
//! at the region's edges), and c = (R/Δ)⁴ (their Section 2.2: c from 0.4 to 1.4 (R/Δ)⁴ keeps
//! the indicator finite over a pixel at any flat point of a discrete field).
//!
//! A. M. Hammond, A. Oskooi, S. G. Johnson, S. E. Ralph, "Photonic topology optimization with
//! semiconductor-foundry design-rule constraints", Opt. Express 29, 23916 (2021),
//! doi:10.1364/OE.431188, Section 3.1, Eqs. 10 to 15: the same constraints as the minimum
//! linewidth (g_LW) and linespacing (g_LS) of a foundry's rules, and the thresholds from the
//! filter radius R and the linewidth l_w or spacing l_s (Eqs. 14 and 15, Qian & Sigmund 2013's
//! Eq. 30): η_e = ¼(l_w/R)² + ½ for l_w ≤ R, l_w/R − ¼(l_w/R)² for R ≤ l_w ≤ 2R, and 1 above;
//! η_d = ½ − ¼(l_s/R)², 1 + ¼(l_s/R)² − l_s/R, and 0 likewise. They are the inverse of
//! [`super::minimum_length`].

use super::Density;
use crate::{Error, Result};

/// The eroded threshold η_e that keeps the solid's features at least `linewidth` wide with a
/// conic filter of radius `radius` (Hammond et al.'s Eq. 14).
///
/// # Errors
///
/// [`Error::InvalidValue`] for a length or radius that isn't positive and finite.
pub fn eroded_threshold(linewidth: f64, radius: f64) -> Result<f64> {
    let x = ratio(linewidth, radius)?;
    Ok(if x <= 1.0 {
        0.25 * x * x + 0.5
    } else if x <= 2.0 {
        x - 0.25 * x * x
    } else {
        1.0
    })
}

/// The dilated threshold η_d that keeps the void's gaps at least `spacing` wide with a conic
/// filter of radius `radius` (Hammond et al.'s Eq. 15): 1 − η_e of the same length.
///
/// # Errors
///
/// As [`eroded_threshold`].
pub fn dilated_threshold(spacing: f64, radius: f64) -> Result<f64> {
    Ok(1.0 - eroded_threshold(spacing, radius)?)
}

fn ratio(length: f64, radius: f64) -> Result<f64> {
    if !(length.is_finite() && length > 0.0 && radius.is_finite() && radius > 0.0) {
        return Err(Error::invalid(
            "length scale",
            format!(
                "the length and the radius must be positive and finite, got {length} and {radius}"
            ),
        ));
    }
    Ok(length / radius)
}

/// The two constraints on a 2D density: its filter and projection (the physical field ρ̄ the
/// indicators weigh), the thresholds, and the indicators' decay c.
#[derive(Clone, Debug, PartialEq)]
pub struct LengthConstraints {
    density: Density,
    eta_e: f64,
    eta_d: f64,
    decay: f64,
}

/// The constraints' values and their gradients in the densities.
#[derive(Clone, Debug, PartialEq)]
pub struct Constrained {
    /// g_LW (the solid's) and g_LS (the void's).
    pub values: [f64; 2],
    /// ∂g_LW/∂ρ and ∂g_LS/∂ρ, one value per pixel each.
    pub gradients: [Vec<f64>; 2],
}

impl LengthConstraints {
    /// The constraints on `density` (2D, with a filter and a projection) for a minimum
    /// `linewidth` and `spacing` in µm, the thresholds from its filter's radius (Eqs. 14 and 15)
    /// and c = (R/Δ)⁴.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a density that isn't 2D or has no filter or no projection, or
    /// a length that isn't positive and finite.
    pub fn new(density: &Density, linewidth: f64, spacing: f64) -> Result<LengthConstraints> {
        let Some(filter) = density.filter() else {
            return Err(Error::invalid(
                "length constraints",
                "the density needs a filter: its radius sets the thresholds",
            ));
        };
        if density.projection().is_none() {
            return Err(Error::invalid(
                "length constraints",
                "the density needs a projection: the indicators weigh its physical field",
            ));
        }
        if density.shape()[2] != 1 {
            return Err(Error::invalid(
                "length constraints",
                "the constraints are written for a 2D region (1 pixel along z)",
            ));
        }
        let radius = filter.radius_um();
        let r = radius / density.pixel_um();
        Ok(LengthConstraints {
            density: density.clone(),
            eta_e: eroded_threshold(linewidth, radius)?,
            eta_d: dilated_threshold(spacing, radius)?,
            decay: r.powi(4),
        })
    }

    /// The same with the indicators' decay `c` in place of (R/Δ)⁴.
    pub fn with_decay(self, c: f64) -> LengthConstraints {
        LengthConstraints { decay: c, ..self }
    }

    /// The thresholds η_e and η_d.
    pub fn thresholds(&self) -> [f64; 2] {
        [self.eta_e, self.eta_d]
    }

    /// The indicators' decay c.
    pub fn decay(&self) -> f64 {
        self.decay
    }

    /// g_LW and g_LS at the densities `rho`, with their gradients.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for densities that aren't one value per pixel.
    pub fn evaluate(&self, rho: &[f64]) -> Result<Constrained> {
        let d = &self.density;
        let [ni, nj, _] = d.shape();
        let n = ni * nj;
        if rho.len() != n {
            return Err(Error::invalid(
                "length constraints",
                format!("expected {n} densities, got {}", rho.len()),
            ));
        }
        let (Some(filter), Some(p)) = (d.filter(), d.projection()) else {
            return Err(Error::invalid(
                "length constraints",
                "the density lost its filter or projection",
            ));
        };
        let filtered = filter.apply(rho)?;
        let at = |i: usize, j: usize| filtered[j * ni + i];
        // the per-pixel gradient by central differences, one-sided at the edges
        let diff = |k: usize, len: usize, get: &dyn Fn(usize) -> f64| -> f64 {
            if len == 1 {
                0.0
            } else if k == 0 {
                get(1) - get(0)
            } else if k == len - 1 {
                get(len - 1) - get(len - 2)
            } else {
                0.5 * (get(k + 1) - get(k - 1))
            }
        };
        let mut gx = vec![0.0; n];
        let mut gy = vec![0.0; n];
        for j in 0..nj {
            for i in 0..ni {
                gx[j * ni + i] = diff(i, ni, &|k| at(k, j));
                gy[j * ni + i] = diff(j, nj, &|k| at(i, k));
            }
        }
        let c = self.decay;
        let inv_n = 1.0 / n as f64;
        let mut values = [0.0f64; 2];
        // ∂g/∂ρ̃ at each pixel (its own terms), and the weights on the gradient's terms
        let mut d_filtered = [vec![0.0; n], vec![0.0; n]];
        let mut wx = [vec![0.0; n], vec![0.0; n]];
        let mut wy = [vec![0.0; n], vec![0.0; n]];
        for q in 0..n {
            let t = filtered[q];
            let bar = p.value(t);
            let dbar = p.derivative(t);
            let e = (-c * (gx[q] * gx[q] + gy[q] * gy[q])).exp();
            // the solid's: I = ρ̄ e, m = min(ρ̃ − η_e, 0); the void's: I = (1 − ρ̄) e, m = min(η_d − ρ̃, 0)
            let terms = [
                (bar, dbar, (t - self.eta_e).min(0.0), 1.0),
                (1.0 - bar, -dbar, (self.eta_d - t).min(0.0), -1.0),
            ];
            for (s, (weight, dweight, m, dm)) in terms.into_iter().enumerate() {
                let m2 = m * m;
                values[s] += inv_n * weight * e * m2;
                // through ρ̄ and through m (dm/dρ̃ = ±1 where m < 0)
                let dm2 = if m < 0.0 { 2.0 * m * dm } else { 0.0 };
                d_filtered[s][q] += inv_n * e * (dweight * m2 + weight * dm2);
                // through e: ∂e/∂(∇ρ̃) = −2c e ∇ρ̃
                let common = inv_n * weight * m2 * e * (-2.0 * c);
                wx[s][q] = common * gx[q];
                wy[s][q] = common * gy[q];
            }
        }
        // the differences' transposes: Σ_q w_q ∂(∇ρ̃)_q/∂ρ̃_k
        for s in 0..2 {
            for j in 0..nj {
                for i in 0..ni {
                    let q = j * ni + i;
                    scatter_diff(i, ni, wx[s][q], &mut |k, v| d_filtered[s][j * ni + k] += v);
                    scatter_diff(j, nj, wy[s][q], &mut |k, v| d_filtered[s][k * ni + i] += v);
                }
            }
        }
        let gradients = [
            filter.transpose(&d_filtered[0])?,
            filter.transpose(&d_filtered[1])?,
        ];
        Ok(Constrained { values, gradients })
    }
}

/// Adds w times the coefficients of the difference at `k` (of `len`) to the pixels they read.
fn scatter_diff(k: usize, len: usize, w: f64, add: &mut dyn FnMut(usize, f64)) {
    if len == 1 || w == 0.0 {
        return;
    }
    if k == 0 {
        add(1, w);
        add(0, -w);
    } else if k == len - 1 {
        add(len - 1, w);
        add(len - 2, -w);
    } else {
        add(k + 1, 0.5 * w);
        add(k - 1, -0.5 * w);
    }
}
