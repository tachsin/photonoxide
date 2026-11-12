//! The Helmholtz-equation density filter, and the double filtering of Christiansen & Sigmund's
//! robust demultiplexer.
//!
//! B. S. Lazarov, O. Sigmund, "Filters in topology optimization based on Helmholtz-type
//! differential equations", Int. J. Numer. Methods Eng. 86, 765 (published online 2010),
//! doi:10.1002/nme.3072, Eqs. 11, 12 and 34: the filtered density solves
//! −r²∇²ρ̃ + ρ̃ = ρ with ∂ρ̃/∂n = 0 on the region's edges, which keeps the volume; r = R/(2√3)
//! for a conic filter's radius R gives a comparable filter (Eq. 34).
//!
//! On a regular grid of pixels with the edges mirrored (the cell-centred second difference with
//! a ghost pixel equal to its neighbour), the discrete operator 1 − r²Δ_h is diagonal in the
//! cosine transform (DCT-II): along an axis of n pixels its eigenvectors are
//! cos(πk(i + ½)/n) and the second difference's eigenvalues −4 sin²(πk/2n)/Δ². So the filter is
//! a transform, a division by 1 + (r/Δ)² Σ 4 sin²(πk/2n) over the axes, and the transform back:
//! exact (no iteration), symmetric (it is its own transpose), and a constant density is kept
//! (k = 0 divides by 1). Each transform is a dense n × n product along its axis: for regions of
//! a few hundred pixels a side, a few milliseconds, and the same bits on any number of threads.
//!
//! R. E. Christiansen, O. Sigmund, J. Opt. Soc. Am. B 38, 496 (2021), doi:10.1364/JOSAB.406048,
//! Section 9 and Eq. 22: the demultiplexer's double filtering, a filter of radius r_f,1 and a
//! projection (β₁, η₁), then a second filter of r_f,2 and a projection at three thresholds η₂,k,
//! the eroded, nominal and dilated realizations of a near-uniform ±8 nm etch. Across a straight
//! edge the second filter's density is ½ e^(−x/r) outside a solid half-space (its Green's
//! function e^(−|x|/r)/2r, integrated), so a threshold η moves the edge by r ln(1/2η): 7.4 nm for
//! r_f = 50 nm and η = 0.3.

use rayon::prelude::*;

use super::Projection;
use crate::{Error, Result};

/// The Helmholtz filter on a region of pixels, its edges mirrored (Lazarov & Sigmund's Eqs. 11
/// and 12).
#[derive(Clone, Debug, PartialEq)]
pub struct HelmholtzFilter {
    shape: [usize; 3],
    radius_um: f64,
    /// Per axis, the orthonormal DCT-II matrix, row k the k-th cosine (empty for an axis one
    /// pixel thick).
    q: [Vec<f64>; 3],
    /// 1 / (1 + (r/Δ)² Σ 4 sin²(πk/2n)) per mode, i fastest.
    scale: Vec<f64>,
}

fn dct(n: usize) -> Vec<f64> {
    let mut q = vec![0.0; n * n];
    for k in 0..n {
        let c = if k == 0 {
            (1.0 / n as f64).sqrt()
        } else {
            (2.0 / n as f64).sqrt()
        };
        for i in 0..n {
            q[k * n + i] =
                c * (std::f64::consts::PI * k as f64 * (i as f64 + 0.5) / n as f64).cos();
        }
    }
    q
}

impl HelmholtzFilter {
    /// The filter on a region of `shape` pixels, each `pixel_um` wide, of radius `radius_um` in
    /// the conic filter's sense: r = R/(2√3) in the equation (Lazarov & Sigmund's Eq. 34;
    /// Christiansen & Sigmund's r_f).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an empty region or a pixel or radius that isn't positive and
    /// finite.
    pub fn new(shape: [usize; 3], pixel_um: f64, radius_um: f64) -> Result<HelmholtzFilter> {
        if shape.contains(&0) || !(pixel_um.is_finite() && pixel_um > 0.0) {
            return Err(Error::invalid(
                "helmholtz filter",
                format!("a region of {shape:?} pixels of {pixel_um} µm"),
            ));
        }
        if !(radius_um.is_finite() && radius_um > 0.0) {
            return Err(Error::invalid(
                "helmholtz filter",
                format!("the radius must be positive and finite, got {radius_um} µm"),
            ));
        }
        let r = radius_um / (2.0 * 3f64.sqrt()) / pixel_um;
        let q = shape.map(|n| if n == 1 { Vec::new() } else { dct(n) });
        let lambda = |n: usize, k: usize| {
            if n == 1 {
                0.0
            } else {
                let s = (std::f64::consts::PI * k as f64 / (2.0 * n as f64)).sin();
                4.0 * s * s
            }
        };
        let [ni, nj, nk] = shape;
        let mut scale = Vec::with_capacity(ni * nj * nk);
        for c in 0..nk {
            for b in 0..nj {
                for a in 0..ni {
                    let l = lambda(ni, a) + lambda(nj, b) + lambda(nk, c);
                    scale.push(1.0 / (1.0 + r * r * l));
                }
            }
        }
        Ok(HelmholtzFilter {
            shape,
            radius_um,
            q,
            scale,
        })
    }

    /// The region's shape in pixels.
    pub fn shape(&self) -> [usize; 3] {
        self.shape
    }

    /// The radius in the conic filter's sense, µm.
    pub fn radius_um(&self) -> f64 {
        self.radius_um
    }

    /// The length r in the equation, µm: R/(2√3).
    pub fn length_um(&self) -> f64 {
        self.radius_um / (2.0 * 3f64.sqrt())
    }

    /// Applies `m` (n × n, row-major) along `axis` of `v`, or its transpose with `transpose`.
    fn along(&self, v: &[f64], axis: usize, transpose: bool) -> Vec<f64> {
        let [ni, nj, nk] = self.shape;
        let n = self.shape[axis];
        if n == 1 {
            return v.to_vec();
        }
        let m = &self.q[axis];
        let stride = [1, ni, ni * nj][axis];
        (0..ni * nj * nk)
            .into_par_iter()
            .map(|p| {
                let pos = (p / stride) % n;
                let base = p - pos * stride;
                (0..n)
                    .map(|t| {
                        let w = if transpose {
                            m[t * n + pos]
                        } else {
                            m[pos * n + t]
                        };
                        w * v[base + t * stride]
                    })
                    .sum()
            })
            .collect()
    }

    /// The filtered densities of `rho`, one per pixel, i fastest.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for densities that aren't one per pixel.
    pub fn apply(&self, rho: &[f64]) -> Result<Vec<f64>> {
        if rho.len() != self.scale.len() {
            return Err(Error::invalid(
                "helmholtz filter",
                format!("expected {} densities, got {}", self.scale.len(), rho.len()),
            ));
        }
        let mut v = rho.to_vec();
        for axis in 0..3 {
            v = self.along(&v, axis, false);
        }
        for (x, s) in v.iter_mut().zip(&self.scale) {
            *x *= s;
        }
        for axis in 0..3 {
            v = self.along(&v, axis, true);
        }
        Ok(v)
    }

    /// The filter's transpose applied to `g`: the filter itself, which is symmetric.
    ///
    /// # Errors
    ///
    /// As [`HelmholtzFilter::apply`].
    pub fn transpose(&self, g: &[f64]) -> Result<Vec<f64>> {
        self.apply(g)
    }
}

/// Christiansen & Sigmund's double filtering: densities ρ filtered (r_f,1) and projected
/// (β₁, η₁), filtered again (r_f,2) and projected (β₂, η₂), and the permittivity
/// ε₀ + ρ̂ (ε₁ − ε₀) a solver sees.
#[derive(Clone, Debug, PartialEq)]
pub struct DoubleFiltered {
    eps: [f64; 2],
    first: HelmholtzFilter,
    project_first: Projection,
    second: HelmholtzFilter,
    project_second: Projection,
}

impl DoubleFiltered {
    /// The pipeline on a region of `shape` pixels, `pixel_um` wide, between permittivities
    /// `eps` (at density 0 and 1), with the filters' radii and the two projections.
    ///
    /// # Errors
    ///
    /// As [`HelmholtzFilter::new`], and for a permittivity that isn't finite.
    pub fn new(
        shape: [usize; 3],
        pixel_um: f64,
        eps: [f64; 2],
        radii_um: [f64; 2],
        projections: [Projection; 2],
    ) -> Result<DoubleFiltered> {
        if !eps.iter().all(|e| e.is_finite()) {
            return Err(Error::invalid(
                "double filtered",
                format!("the permittivities must be finite, got {eps:?}"),
            ));
        }
        Ok(DoubleFiltered {
            eps,
            first: HelmholtzFilter::new(shape, pixel_um, radii_um[0])?,
            project_first: projections[0],
            second: HelmholtzFilter::new(shape, pixel_um, radii_um[1])?,
            project_second: projections[1],
        })
    }

    /// The same with the second projection at threshold `eta` (its β kept).
    ///
    /// # Errors
    ///
    /// As [`Projection::new`].
    pub fn at_threshold(&self, eta: f64) -> Result<DoubleFiltered> {
        Ok(DoubleFiltered {
            project_second: Projection::new(self.project_second.beta(), eta)?,
            ..self.clone()
        })
    }

    /// The eroded, nominal and dilated realizations: the second projection at 1 − `eta`, ½ and
    /// `eta`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an η outside (0, ½).
    pub fn robust(&self, eta: f64) -> Result<[DoubleFiltered; 3]> {
        if !(eta > 0.0 && eta < 0.5) {
            return Err(Error::invalid(
                "double filtered",
                format!("η must be in (0, ½), got {eta}"),
            ));
        }
        Ok([
            self.at_threshold(1.0 - eta)?,
            self.at_threshold(0.5)?,
            self.at_threshold(eta)?,
        ])
    }

    /// The two projections.
    pub fn projections(&self) -> [Projection; 2] {
        [self.project_first, self.project_second]
    }

    /// The two filters.
    pub fn filters(&self) -> [&HelmholtzFilter; 2] {
        [&self.first, &self.second]
    }

    /// The pipeline's stages at `rho`: the first filtered, first projected, second filtered and
    /// physical densities.
    fn stages(&self, rho: &[f64]) -> Result<[Vec<f64>; 4]> {
        let f1 = self.first.apply(rho)?;
        let p1: Vec<f64> = f1.iter().map(|&x| self.project_first.value(x)).collect();
        let f2 = self.second.apply(&p1)?;
        let p2: Vec<f64> = f2.iter().map(|&x| self.project_second.value(x)).collect();
        Ok([f1, p1, f2, p2])
    }

    /// The physical densities ρ̂ of `rho`.
    ///
    /// # Errors
    ///
    /// As [`HelmholtzFilter::apply`].
    pub fn physical(&self, rho: &[f64]) -> Result<Vec<f64>> {
        Ok(self.stages(rho)?[3].clone())
    }

    /// The relative permittivity a solver sees, ε₀ + ρ̂ (ε₁ − ε₀).
    ///
    /// # Errors
    ///
    /// As [`HelmholtzFilter::apply`].
    pub fn permittivity(&self, rho: &[f64]) -> Result<Vec<f64>> {
        let [e0, e1] = self.eps;
        Ok(self
            .physical(rho)?
            .into_iter()
            .map(|x| e0 + x * (e1 - e0))
            .collect())
    }

    /// ∂F/∂ρ from ∂F/∂ρ̂ at each pixel: back through the second projection, the second filter,
    /// the first projection and the first filter (each filter its own transpose).
    ///
    /// # Errors
    ///
    /// As [`HelmholtzFilter::apply`].
    pub fn pullback(&self, rho: &[f64], d_physical: &[f64]) -> Result<Vec<f64>> {
        let [f1, _, f2, _] = self.stages(rho)?;
        if d_physical.len() != f2.len() {
            return Err(Error::invalid(
                "double filtered",
                format!("expected {} gradients, got {}", f2.len(), d_physical.len()),
            ));
        }
        let g: Vec<f64> = d_physical
            .iter()
            .zip(&f2)
            .map(|(g, &x)| g * self.project_second.derivative(x))
            .collect();
        let g = self.second.transpose(&g)?;
        let g: Vec<f64> = g
            .iter()
            .zip(&f1)
            .map(|(g, &x)| g * self.project_first.derivative(x))
            .collect();
        self.first.transpose(&g)
    }

    /// ∂F/∂ρ from ∂F/∂ε at each pixel: [`DoubleFiltered::pullback`] of (ε₁ − ε₀) ∂F/∂ε.
    ///
    /// # Errors
    ///
    /// As [`DoubleFiltered::pullback`].
    pub fn gradient(&self, rho: &[f64], d_eps: &[f64]) -> Result<Vec<f64>> {
        let s = self.eps[1] - self.eps[0];
        let d: Vec<f64> = d_eps.iter().map(|g| g * s).collect();
        self.pullback(rho, &d)
    }
}
