//! Lithography inside the loop: what a lithography step prints from a mask, by a proxy (a
//! Gaussian blur of the mask and a threshold of the blurred image), and a figure of merit's
//! gradient through it.
//!
//! S. Khan (S. Raza at Optica and on Crossref), M. Hammood, N. A. F. Jaeger, L. Chrostowski,
//! "Fabrication-aware inverse design with shape optimization for photonic integrated circuits",
//! Opt. Lett. 50, 117 (2024), doi:10.1364/OL.543961 (read in the manuscript, arXiv:2410.07353):
//! the figure of merit is computed on the lithography-predicted geometry, and its gradient is the
//! adjoint's sensitivity there times the lithography model's Jacobian (their Eq. 2). Their models
//! are the University of British Columbia's DUV model in Mentor Graphics Calibre and PreFab's
//! neural network for electron-beam lithography; neither is open, and the paper states no
//! parameter of either. The proxy here is the simplest differentiable model of the same kind:
//!
//! - **the blur:** the mask m (1 where the pattern is, 0 elsewhere, a fill fraction between) is
//!   convolved with a normalized Gaussian of standard deviation σ in the plane, I = G_σ ∗ m. The
//!   mask is taken as constant over each pixel, so each pixel's weight is the Gaussian's integral
//!   over it, ½ [erf((k + ½)Δ/√2σ) − erf((k − ½)Δ/√2σ)] along each axis (the Gaussian is
//!   separable): the blurred value at a pixel's centre is then the exact convolution of the
//!   pixelated mask, with no sampling error, and a solid half-plane whose edge lies on a pixel
//!   boundary blurs to exactly ½ erfc(x/√2σ);
//! - **the threshold:** the printed pattern is where I exceeds t (a resist's dose to clear), by
//!   Hammond et al. 2025's subpixel-smoothed projection ([`SmoothedProjection`]) at η = t, sharp
//!   (β = ∞) by default: each pixel near the contour I = t is filled by the share of it on the
//!   printed side, so the printed pattern stays differentiable in the mask;
//! - **the gradient** (Khan et al.'s Eq. 2): ∂F/∂m is the projection's transpose applied to ∂F/∂p
//!   (p the printed pattern), then the blur's transpose; the Gaussian is symmetric, so its
//!   transpose is the same convolution, renormalized at the region's edges as the blur is.
//!
//! On a straight edge the proxy moves the printed edge by √2 σ erfc⁻¹(2t) into the void
//! ([`Lithography::edge_shift`]), outwards for t < ½; an isolated line of width w peaks at
//! erf(w/2√2σ) and is lost when that falls below t ([`Lithography::line_width`]); a convex
//! corner's blurred value is a quarter of the plane's, so its tip recedes (corner rounding). These
//! are the variations Khan et al. list (corner rounding, line shortening, proximity effects); the
//! proxy has no sidewall roughness and no randomness.

use rayon::prelude::*;

use super::smoothed::SmoothedProjection;
use super::{Edge, check_shape, coords, len};
use crate::{Error, Result};

/// The blur's reach, in standard deviations: the Gaussian's mass beyond it, erfc(9/√2) ≈ 2e-19,
/// is below a double's round-off.
const REACH: f64 = 9.0;

/// A normalized Gaussian blur of standard deviation σ on a region of pixels, in the plane (x and
/// y; a region's layers along z are each blurred alone), each pixel's weight the Gaussian's
/// integral over it.
#[derive(Clone, Debug, PartialEq)]
pub struct GaussianBlur {
    shape: [usize; 3],
    pixel_um: f64,
    sigma_um: f64,
    edge: Edge,
    /// The weight of a pixel k pixels away, k = 0 … reach, the whole kernel summing to 1.
    weights: Vec<f64>,
    /// Along x and y, each index's sum of the weights within the region (all 1 when padded).
    norms: [Vec<f64>; 2],
}

impl GaussianBlur {
    /// The blur of standard deviation `sigma_um` on a region of `shape` pixels, each `pixel_um`
    /// wide, its edges treated as `edge` ([`Edge::Renormalized`]: only the region's pixels are
    /// averaged, which keeps a guide that crosses an edge; [`Edge::Padded`]: the mask continues
    /// outside with that value).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an empty region, a pixel or σ that isn't positive and finite,
    /// or a padding outside [0, 1].
    pub fn new(shape: [usize; 3], pixel_um: f64, sigma_um: f64, edge: Edge) -> Result<Self> {
        check_shape(shape, pixel_um)?;
        if !(sigma_um.is_finite() && sigma_um > 0.0) {
            return Err(Error::invalid(
                "lithography blur",
                format!("σ must be positive and finite, got {sigma_um} µm"),
            ));
        }
        if let Edge::Padded(v) = edge
            && !(0.0..=1.0).contains(&v)
        {
            return Err(Error::invalid(
                "lithography blur",
                format!("the padding must be in [0, 1], got {v}"),
            ));
        }
        let reach = (REACH * sigma_um / pixel_um).ceil() as usize;
        let a = pixel_um / (std::f64::consts::SQRT_2 * sigma_um);
        // the Gaussian's integral over pixel k, by erfc for k > 0 so that the tails keep their digits
        let mut weights: Vec<f64> = (0..=reach)
            .map(|k| {
                if k == 0 {
                    libm::erf(0.5 * a)
                } else {
                    let k = k as f64;
                    0.5 * (libm::erfc((k - 0.5) * a) - libm::erfc((k + 0.5) * a))
                }
            })
            .collect();
        let total = weights[0] + 2.0 * weights[1..].iter().sum::<f64>();
        for w in &mut weights {
            *w /= total;
        }
        let norm = |n: usize| -> Vec<f64> {
            (0..n)
                .map(|i| match edge {
                    Edge::Padded(_) => 1.0,
                    Edge::Renormalized => (0..n)
                        .filter(|&j| i.abs_diff(j) <= reach)
                        .map(|j| weights[i.abs_diff(j)])
                        .sum(),
                })
                .collect()
        };
        let norms = [norm(shape[0]), norm(shape[1])];
        Ok(GaussianBlur {
            shape,
            pixel_um,
            sigma_um,
            edge,
            weights,
            norms,
        })
    }

    /// The region's shape in pixels.
    pub fn shape(&self) -> [usize; 3] {
        self.shape
    }

    /// The pixel's width, µm.
    pub fn pixel_um(&self) -> f64 {
        self.pixel_um
    }

    /// The standard deviation σ, µm.
    pub fn sigma_um(&self) -> f64 {
        self.sigma_um
    }

    /// How the edges are treated.
    pub fn edge(&self) -> Edge {
        self.edge
    }

    fn check_len(&self, v: &[f64]) -> Result<()> {
        if v.len() != len(self.shape) {
            return Err(Error::invalid(
                "lithography blur",
                format!(
                    "expected one value per pixel, {}, got {}",
                    len(self.shape),
                    v.len()
                ),
            ));
        }
        Ok(())
    }

    /// One pass along `axis` (0 for x, 1 for y): each pixel's weighted sum of its neighbours along
    /// the axis, the pixels outside the region taken as `pad` (none: left out). Forward, each sum
    /// is divided by the pixel's own norm; transposed, each neighbour's value by the neighbour's.
    fn pass(&self, v: &[f64], axis: usize, pad: Option<f64>, transposed: bool) -> Vec<f64> {
        let n = self.shape[axis];
        if n == 1 {
            return v.to_vec();
        }
        let stride = if axis == 0 { 1 } else { self.shape[0] };
        let reach = self.weights.len() - 1;
        let norms = &self.norms[axis];
        (0..v.len())
            .into_par_iter()
            .map(|p| {
                let i = coords(self.shape, p)[axis];
                let base = p - i * stride;
                let mut sum = 0.0;
                for k in i.saturating_sub(reach)..=(i + reach).min(n - 1) {
                    let x = v[base + k * stride];
                    let w = self.weights[i.abs_diff(k)];
                    sum += if transposed { w * x / norms[k] } else { w * x };
                }
                if let Some(pad) = pad {
                    // the pixels beyond each end, whose weights are the tails left
                    let below = reach.saturating_sub(i);
                    let above = (i + reach + 1).saturating_sub(n);
                    let tail: f64 = (i + 1..=i + below).map(|d| self.weights[d]).sum::<f64>()
                        + (n - i..n - i + above).map(|d| self.weights[d]).sum::<f64>();
                    sum += pad * tail;
                }
                if transposed { sum } else { sum / norms[i] }
            })
            .collect()
    }

    /// The blurred mask, G_σ ∗ m at each pixel's centre.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a mask that isn't one value per pixel.
    pub fn apply(&self, mask: &[f64]) -> Result<Vec<f64>> {
        self.check_len(mask)?;
        let pad = match self.edge {
            Edge::Padded(v) => Some(v),
            Edge::Renormalized => None,
        };
        let x = self.pass(mask, 0, pad, false);
        Ok(self.pass(&x, 1, pad, false))
    }

    /// The blur's transpose applied to `g`: the gradient with respect to the mask from `g`, the
    /// gradient with respect to the blurred mask. Padded, the blur is affine and this is its
    /// linear part's transpose.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a gradient that isn't one value per pixel.
    pub fn transpose(&self, g: &[f64]) -> Result<Vec<f64>> {
        self.check_len(g)?;
        let pad = match self.edge {
            Edge::Padded(_) => Some(0.0),
            Edge::Renormalized => None,
        };
        let y = self.pass(g, 1, pad, true);
        Ok(self.pass(&y, 0, pad, true))
    }
}

/// erfc⁻¹(y) for y in (0, 2): bisection on [−6, 6], then two Newton steps.
fn erfc_inv(y: f64) -> f64 {
    let (mut lo, mut hi) = (-6.0f64, 6.0f64);
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if libm::erfc(mid) > y {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let mut x = 0.5 * (lo + hi);
    for _ in 0..2 {
        let slope = -2.0 / std::f64::consts::PI.sqrt() * libm::exp(-x * x);
        if slope != 0.0 {
            x -= (libm::erfc(x) - y) / slope;
        }
    }
    x
}

/// A lithography proxy: a [`GaussianBlur`] of the mask and a threshold of the blurred image at
/// t, by the subpixel-smoothed projection (sharp unless β is given).
#[derive(Clone, Debug, PartialEq)]
pub struct Lithography {
    blur: GaussianBlur,
    resist: SmoothedProjection,
}

impl Lithography {
    /// The proxy of blur `sigma_um` and threshold `threshold` on a region of `shape` pixels, each
    /// `pixel_um` wide, its edges treated as `edge`; the threshold sharp (β = ∞).
    ///
    /// # Errors
    ///
    /// As [`GaussianBlur::new`], and [`Error::InvalidValue`] for a threshold outside (0, 1).
    pub fn new(
        shape: [usize; 3],
        pixel_um: f64,
        sigma_um: f64,
        threshold: f64,
        edge: Edge,
    ) -> Result<Lithography> {
        Ok(Lithography {
            blur: GaussianBlur::new(shape, pixel_um, sigma_um, edge)?,
            resist: SmoothedProjection::new(f64::INFINITY, threshold)?,
        })
    }

    /// The same proxy with the threshold's sharpness `beta`, finite or infinite.
    ///
    /// # Errors
    ///
    /// As [`SmoothedProjection::new`].
    pub fn with_beta(self, beta: f64) -> Result<Lithography> {
        Ok(Lithography {
            resist: SmoothedProjection::new(beta, self.resist.eta())?,
            ..self
        })
    }

    /// The blur.
    pub fn blur(&self) -> &GaussianBlur {
        &self.blur
    }

    /// The threshold t.
    pub fn threshold(&self) -> f64 {
        self.resist.eta()
    }

    /// The threshold's sharpness β (infinite unless given).
    pub fn beta(&self) -> f64 {
        self.resist.beta()
    }

    /// The printed pattern of `mask`, each pixel's share in [0, 1].
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a mask that isn't one value per pixel.
    pub fn print(&self, mask: &[f64]) -> Result<Vec<f64>> {
        let blurred = self.blur.apply(mask)?;
        self.resist
            .apply(self.blur.shape, self.blur.pixel_um, &blurred)
    }

    /// ∂F/∂m from `d_printed`, ∂F/∂p at each pixel: the threshold's transpose at the blurred
    /// mask, then the blur's (Khan et al.'s Eq. 2).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a mask or gradient that isn't one value per pixel.
    pub fn pullback(&self, mask: &[f64], d_printed: &[f64]) -> Result<Vec<f64>> {
        let blurred = self.blur.apply(mask)?;
        let d_blurred =
            self.resist
                .transpose(self.blur.shape, self.blur.pixel_um, &blurred, d_printed)?;
        self.blur.transpose(&d_blurred)
    }

    /// How far the proxy moves a straight edge into the void, µm: √2 σ erfc⁻¹(2t), where a solid
    /// half-plane's blur, ½ erfc(x/√2σ), crosses t. Positive (the pattern grows) for t < ½.
    pub fn edge_shift(sigma_um: f64, threshold: f64) -> f64 {
        std::f64::consts::SQRT_2 * sigma_um * erfc_inv(2.0 * threshold)
    }

    /// The printed width of an isolated line of width `width_um`, µm: where its blur,
    /// ½ [erf((w/2 − x)/√2σ) + erf((w/2 + x)/√2σ)], crosses t, by bisection; 0 where the peak,
    /// erf(w/2√2σ), doesn't reach t (the line is lost). A gap of width s in solid prints as a
    /// line of width s at threshold 1 − t.
    pub fn line_width(width_um: f64, sigma_um: f64, threshold: f64) -> f64 {
        let s = std::f64::consts::SQRT_2 * sigma_um;
        let h = 0.5 * width_um;
        let blur = |x: f64| 0.5 * (libm::erf((h - x) / s) + libm::erf((h + x) / s));
        if blur(0.0) <= threshold {
            return 0.0;
        }
        let (mut lo, mut hi) = (0.0, h + REACH * sigma_um);
        for _ in 0..100 {
            let mid = 0.5 * (lo + hi);
            if blur(mid) > threshold {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        lo + hi
    }
}
