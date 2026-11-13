//! Hammond et al. 2025's subpixel-smoothed projection (SSP): the projected density as if the
//! filtered density's level set were smoothed over a sphere a little wider than a pixel, so that
//! the design stays differentiable at β = ∞ (a level set) and well conditioned at large β.
//!
//! A. M. Hammond, A. Oskooi, I. M. Hammond, M. Chen, S. E. Ralph, S. G. Johnson, "Unifying and
//! accelerating level-set and density-based topology optimization by subpixel-smoothed
//! projection", Opt. Express 33, 33620 (2025), doi:10.1364/OE.563512, Section 3 and Appendix A.
//!
//! At each pixel the filtered density ρ̃ and its gradient (centred differences, one-sided at the
//! region's edges) give the distance to the ρ̃ = η level set, d = (η − ρ̃)/‖∇ρ̃‖ (their Eq. 7),
//! positive on the void side. Within R̂ = 0.55 Δx of it the value is
//! (1 − F(d)) ρ̂₋ + F(d) ρ̂₊ (Eq. 11), with ρ̂± = P(ρ̃ ± R̂ ‖∇ρ̃‖ F(∓d)) (Eq. 12) and F the
//! twice-differentiable fill F₂ of Eq. 10, the fill of a plane by the parabolic kernel of Eq. 23;
//! elsewhere it is the projection P(ρ̃) itself (Eq. 3). At β = ∞, P is the step at η and the value
//! near the level set is F(d) alone.

use crate::{Error, Result};

/// R̂ over the pixel: their choice, a little more than half a pixel so that every pixel near an
/// interface has a gradient.
pub const RADIUS: f64 = 0.55;

/// Their Eq. 10's fill F₂ at t = d/R̂: ½ − (15/16) t + (5/8) t³ − (3/16) t⁵ for |t| ≤ 1, 0 above,
/// 1 below; the share of a sphere of radius R̂ on the solid side of a plane at distance d from
/// its centre, weighted by the parabolic kernel (1 − (r/R̂)²).
pub fn fill(t: f64) -> f64 {
    if t >= 1.0 {
        0.0
    } else if t <= -1.0 {
        1.0
    } else {
        let t2 = t * t;
        0.5 + t * (-15.0 / 16.0 + t2 * (5.0 / 8.0 - 3.0 / 16.0 * t2))
    }
}

/// dF₂/dt: −(15/16)(1 − t²)², 0 outside [−1, 1].
pub fn fill_slope(t: f64) -> f64 {
    if t.abs() >= 1.0 {
        0.0
    } else {
        let u = 1.0 - t * t;
        -15.0 / 16.0 * u * u
    }
}

/// The subpixel-smoothed projection of sharpness β (finite or infinite) at threshold η.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SmoothedProjection {
    beta: f64,
    eta: f64,
}

/// A pixel's value and its derivatives in ρ̃ and ‖∇ρ̃‖.
struct Local {
    value: f64,
    by_value: f64,
    by_norm: f64,
}

impl SmoothedProjection {
    /// The projection of sharpness `beta`, positive and possibly infinite (`f64::INFINITY`, the
    /// level set), at threshold `eta`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a β that isn't positive (or is NaN), or an η outside (0, 1).
    pub fn new(beta: f64, eta: f64) -> Result<SmoothedProjection> {
        if beta.is_nan() || beta <= 0.0 {
            return Err(Error::invalid(
                "smoothed projection",
                format!("β must be positive, infinite for a level set, got {beta}"),
            ));
        }
        if !(0.0 < eta && eta < 1.0) {
            return Err(Error::invalid(
                "smoothed projection",
                format!("η must be in (0, 1), got {eta}"),
            ));
        }
        Ok(SmoothedProjection { beta, eta })
    }

    /// The sharpness β; infinite for a level set.
    pub fn beta(&self) -> f64 {
        self.beta
    }

    /// The threshold η.
    pub fn eta(&self) -> f64 {
        self.eta
    }

    /// The projection P(x) (their Eq. 3), the step at η when β is infinite (½ on it).
    pub fn project(&self, x: f64) -> f64 {
        if self.beta.is_infinite() {
            if x > self.eta {
                1.0
            } else if x < self.eta {
                0.0
            } else {
                0.5
            }
        } else {
            let (b, e) = (self.beta, self.eta);
            (crate::portable::tanh(b * e) + crate::portable::tanh(b * (x - e)))
                / (crate::portable::tanh(b * e) + crate::portable::tanh(b * (1.0 - e)))
        }
    }

    /// dP/dx: 0 everywhere for the step.
    fn slope(&self, x: f64) -> f64 {
        if self.beta.is_infinite() {
            0.0
        } else {
            let (b, e) = (self.beta, self.eta);
            let sech = 1.0 / crate::portable::cosh(b * (x - e));
            b * sech * sech / (crate::portable::tanh(b * e) + crate::portable::tanh(b * (1.0 - e)))
        }
    }

    /// A pixel's value at filtered density `s` and gradient norm `g` with smoothing radius `r`,
    /// and its derivatives in `s` and `g`.
    fn local(&self, s: f64, g: f64, r: f64) -> Local {
        let far = |p: &Self| Local {
            value: p.project(s),
            by_value: p.slope(s),
            by_norm: 0.0,
        };
        if g.is_nan() || g <= 0.0 {
            return far(self);
        }
        let d = (self.eta - s) / g;
        let t = d / r;
        if t.abs() >= 1.0 {
            return far(self);
        }
        let f = fill(t);
        let df = fill_slope(t) / r;
        let up = s + r * g * (1.0 - f);
        let down = s - r * g * f;
        let (pu, pd) = (self.project(up), self.project(down));
        let (su, sd) = (self.slope(up), self.slope(down));
        // d's derivatives, and each point's through d and directly
        let (dd_s, dd_g) = (-1.0 / g, -d / g);
        let up_s = 1.0 - r * g * df * dd_s;
        let up_g = r * (1.0 - f) - r * g * df * dd_g;
        let down_s = 1.0 - r * g * df * dd_s;
        let down_g = -r * f - r * g * df * dd_g;
        Local {
            value: (1.0 - f) * pd + f * pu,
            by_value: df * dd_s * (pu - pd) + (1.0 - f) * sd * down_s + f * su * up_s,
            by_norm: df * dd_g * (pu - pd) + (1.0 - f) * sd * down_g + f * su * up_g,
        }
    }

    /// The smoothed projection of `filtered`, ρ̃ on a region of `shape` pixels (i fastest, then
    /// j, then k) each `pixel_um` wide.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for values that aren't one per pixel or a pixel that isn't
    /// positive.
    pub fn apply(&self, shape: [usize; 3], pixel_um: f64, filtered: &[f64]) -> Result<Vec<f64>> {
        check(shape, pixel_um, filtered.len())?;
        let r = RADIUS * pixel_um;
        Ok((0..filtered.len())
            .map(|p| {
                let g = gradient(shape, pixel_um, filtered, p);
                let norm = (g[0] * g[0] + g[1] * g[1] + g[2] * g[2]).sqrt();
                self.local(filtered[p], norm, r).value
            })
            .collect())
    }

    /// ∂F/∂ρ̃ from `d_projected`, ∂F/∂ρ̃̂ at each pixel: the transpose of [`apply`]'s Jacobian at
    /// `filtered`, through each pixel's value and the centred differences of its gradient.
    ///
    /// [`apply`]: SmoothedProjection::apply
    ///
    /// # Errors
    ///
    /// As [`SmoothedProjection::apply`], and for a gradient that isn't one per pixel.
    pub fn transpose(
        &self,
        shape: [usize; 3],
        pixel_um: f64,
        filtered: &[f64],
        d_projected: &[f64],
    ) -> Result<Vec<f64>> {
        check(shape, pixel_um, filtered.len())?;
        if d_projected.len() != filtered.len() {
            return Err(Error::invalid(
                "smoothed projection",
                format!(
                    "the gradient has {} values for {} pixels",
                    d_projected.len(),
                    filtered.len()
                ),
            ));
        }
        let r = RADIUS * pixel_um;
        let mut out = vec![0.0; filtered.len()];
        for (p, &w) in d_projected.iter().enumerate() {
            if w == 0.0 {
                continue;
            }
            let g = gradient(shape, pixel_um, filtered, p);
            let norm = (g[0] * g[0] + g[1] * g[1] + g[2] * g[2]).sqrt();
            let l = self.local(filtered[p], norm, r);
            out[p] += w * l.by_value;
            if l.by_norm == 0.0 || norm == 0.0 {
                continue;
            }
            for (axis, &ga) in g.iter().enumerate() {
                let c = w * l.by_norm * ga / norm;
                for (q, weight) in stencil(shape, pixel_um, p, axis) {
                    out[q] += c * weight;
                }
            }
        }
        Ok(out)
    }
}

fn check(shape: [usize; 3], pixel_um: f64, len: usize) -> Result<()> {
    if !(pixel_um.is_finite() && pixel_um > 0.0) {
        return Err(Error::invalid(
            "smoothed projection",
            format!("the pixel must be positive, got {pixel_um}"),
        ));
    }
    if shape.iter().product::<usize>() != len || len == 0 {
        return Err(Error::invalid(
            "smoothed projection",
            format!("{len} values for a region of {shape:?} pixels"),
        ));
    }
    Ok(())
}

/// The difference stencil of axis `axis` at pixel `p`: the pixels and their weights, centred
/// inside and one-sided at the edges; none along an axis of one pixel.
fn stencil(shape: [usize; 3], h: f64, p: usize, axis: usize) -> Vec<(usize, f64)> {
    let n = shape[axis];
    if n < 2 {
        return Vec::new();
    }
    let stride = match axis {
        0 => 1,
        1 => shape[0],
        _ => shape[0] * shape[1],
    };
    let k = (p / stride) % n;
    if k == 0 {
        vec![(p + stride, 1.0 / h), (p, -1.0 / h)]
    } else if k == n - 1 {
        vec![(p, 1.0 / h), (p - stride, -1.0 / h)]
    } else {
        vec![(p + stride, 0.5 / h), (p - stride, -0.5 / h)]
    }
}

/// ∇ρ̃ at pixel `p`.
fn gradient(shape: [usize; 3], h: f64, values: &[f64], p: usize) -> [f64; 3] {
    let mut g = [0.0; 3];
    for (axis, ga) in g.iter_mut().enumerate() {
        *ga = stencil(shape, h, p, axis)
            .iter()
            .map(|&(q, w)| w * values[q])
            .sum();
    }
    g
}
