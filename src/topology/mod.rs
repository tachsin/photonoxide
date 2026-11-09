//! Densities for inverse design: a design region's pixels, each with a density ρ in [0, 1],
//! filtered and projected to the permittivity a solver sees, and the solver's adjoint gradient
//! pulled back to the densities.
//!
//! F. Wang, B. S. Lazarov, O. Sigmund, "On projection methods, convergence and robust
//! formulations in topology optimization", Struct. Multidiscip. Optim. 43, 767 (published
//! online 2010), doi:10.1007/s00158-010-0602-y, Section 3:
//!
//! - **the density filter** (their Eqs. 4 and 5, the filter of Bruns and Tortorelli and of
//!   Bourdin): ρ̃ᵢ = Σⱼ w(xⱼ) ρⱼ / Σⱼ w(xⱼ), with w(xⱼ) = R − |xⱼ − xᵢ| over the pixels whose
//!   centres lie within the radius R of pixel i's (the pixels' volumes are equal, so they
//!   cancel);
//! - **the threshold projection** (their Eq. 9, the tanh form of Xu et al.'s): ρ̂ = (tanh βη +
//!   tanh β(ρ̃ − η)) / (tanh βη + tanh β(1 − η)), which tends to a step at η as β grows;
//! - **the chain rule** (their Eq. 7): ∂f/∂ρⱼ = Σᵢ (∂f/∂ρ̂ᵢ)(∂ρ̂ᵢ/∂ρ̃ᵢ)(∂ρ̃ᵢ/∂ρⱼ), the
//!   filter's transpose applied to the projection's derivative times the solver's gradient.
//!
//! R. E. Christiansen, O. Sigmund, "Inverse design in photonics by topology optimization:
//! tutorial", J. Opt. Soc. Am. B 38, 496 (2021), doi:10.1364/JOSAB.406048, Eq. 18 and the
//! interpolation below it, ε(ρ̂) = ε₀ + ρ̂ (ε₁ − ε₀), are the same projection and the permittivity
//! a solver is given.
//!
//! A [`Density`] holds the region's shape and pixel, the two permittivities, an optional
//! [`ConicFilter`] and an optional [`Projection`]. [`Density::permittivity`] gives what a solver
//! sees, and [`Density::gradient`] takes the solver's gradient with respect to it, ∂F/∂ε per
//! pixel (2D FDFD's `Solver2d::mode_power_gradient` restricted to the region by a
//! [`Placement`]; FDTD's `Simulation::gradient` gives ∂F/∂ρ̂ directly, for
//! [`Density::pullback`]), back to ∂F/∂ρ. Pixels are ordered i fastest, then j, then k, as the
//! solvers order their cells.

use rayon::prelude::*;

use crate::{Error, Result};

#[path = "../cases/topology/checks.rs"]
pub(crate) mod checks;
#[cfg(test)]
mod tests;

/// How the filter treats the region's edges.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Edge {
    /// Only the region's own pixels are averaged, the weights renormalized at each pixel: Wang et
    /// al.'s Eq. 4 with the neighbourhood cut off at the region's edges.
    Renormalized,
    /// The region continues outside with this fixed density, which the filter averages in and
    /// the optimization doesn't vary: a guide entering the region (1) or the cladding around it
    /// (0).
    Padded(f64),
}

/// The conic (linear hat) density filter of radius R on a region of pixels (Wang et al. 2010,
/// Eqs. 4 and 5).
#[derive(Clone, Debug, PartialEq)]
pub struct ConicFilter {
    shape: [usize; 3],
    radius_um: f64,
    edge: Edge,
    /// Each neighbour's offset in pixels and its weight R − d, in µm, d < R.
    offsets: Vec<([isize; 3], f64)>,
    /// Each pixel's Σ w: its neighbours in the region's ([`Edge::Renormalized`]), all of them
    /// ([`Edge::Padded`]).
    norms: Vec<f64>,
}

fn check_shape(shape: [usize; 3], pixel_um: f64) -> Result<()> {
    if shape.contains(&0) {
        return Err(Error::invalid(
            "density",
            format!("a region needs at least one pixel along each axis, got {shape:?}"),
        ));
    }
    if !(pixel_um.is_finite() && pixel_um > 0.0) {
        return Err(Error::invalid(
            "density",
            format!("the pixel must be positive and finite, got {pixel_um} µm"),
        ));
    }
    Ok(())
}

fn len(shape: [usize; 3]) -> usize {
    shape.iter().product()
}

/// Pixel `p`'s coordinates (i, j, k).
fn coords(shape: [usize; 3], p: usize) -> [usize; 3] {
    let [nx, ny, _] = shape;
    [p % nx, (p / nx) % ny, p / (nx * ny)]
}

/// The index of the pixel `offset` away from `at`, if it is in the region.
fn neighbour(shape: [usize; 3], at: [usize; 3], offset: [isize; 3]) -> Option<usize> {
    let mut c = [0usize; 3];
    for a in 0..3 {
        let v = at[a] as isize + offset[a];
        if v < 0 || v >= shape[a] as isize {
            return None;
        }
        c[a] = v as usize;
    }
    Some((c[2] * shape[1] + c[1]) * shape[0] + c[0])
}

impl ConicFilter {
    /// The filter of radius `radius_um` on a region of `shape` pixels (1 along z for 2D, along
    /// y and z for 1D), each `pixel_um` wide, its edges treated as `edge`. A radius below a
    /// pixel leaves the density as it is.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an empty region, a pixel or radius that isn't positive and
    /// finite, or a padding density outside [0, 1].
    pub fn new(shape: [usize; 3], pixel_um: f64, radius_um: f64, edge: Edge) -> Result<Self> {
        check_shape(shape, pixel_um)?;
        if !(radius_um.is_finite() && radius_um > 0.0) {
            return Err(Error::invalid(
                "density filter",
                format!("the radius must be positive and finite, got {radius_um} µm"),
            ));
        }
        if let Edge::Padded(v) = edge
            && !(0.0..=1.0).contains(&v)
        {
            return Err(Error::invalid(
                "density filter",
                format!("the padding density must be in [0, 1], got {v}"),
            ));
        }
        // the reach in pixels along each axis, none along an axis the region is flat in
        let reach: Vec<isize> = shape
            .iter()
            .map(|&n| {
                if n == 1 {
                    0
                } else {
                    (radius_um / pixel_um).ceil() as isize
                }
            })
            .collect();
        let mut offsets = Vec::new();
        for dk in -reach[2]..=reach[2] {
            for dj in -reach[1]..=reach[1] {
                for di in -reach[0]..=reach[0] {
                    let d = pixel_um * ((di * di + dj * dj + dk * dk) as f64).sqrt();
                    if d < radius_um {
                        offsets.push(([di, dj, dk], radius_um - d));
                    }
                }
            }
        }
        let total: f64 = offsets.iter().map(|o| o.1).sum();
        let norms = match edge {
            Edge::Padded(_) => vec![total; len(shape)],
            Edge::Renormalized => (0..len(shape))
                .into_par_iter()
                .map(|p| {
                    let at = coords(shape, p);
                    offsets
                        .iter()
                        .filter(|(o, _)| neighbour(shape, at, *o).is_some())
                        .map(|o| o.1)
                        .sum()
                })
                .collect(),
        };
        Ok(ConicFilter {
            shape,
            radius_um,
            edge,
            offsets,
            norms,
        })
    }

    /// The region's shape in pixels.
    pub fn shape(&self) -> [usize; 3] {
        self.shape
    }

    /// The radius, µm.
    pub fn radius_um(&self) -> f64 {
        self.radius_um
    }

    /// How the edges are treated.
    pub fn edge(&self) -> Edge {
        self.edge
    }

    fn check_len(&self, v: &[f64]) -> Result<()> {
        if v.len() != len(self.shape) {
            return Err(Error::invalid(
                "density filter",
                format!(
                    "expected one value per pixel, {}, got {}",
                    len(self.shape),
                    v.len()
                ),
            ));
        }
        Ok(())
    }

    /// The filtered density ρ̃ of `rho`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a density that isn't one value per pixel.
    pub fn apply(&self, rho: &[f64]) -> Result<Vec<f64>> {
        self.check_len(rho)?;
        let pad = match self.edge {
            Edge::Padded(v) => v,
            Edge::Renormalized => 0.0,
        };
        Ok((0..rho.len())
            .into_par_iter()
            .map(|p| {
                let at = coords(self.shape, p);
                let sum: f64 = self
                    .offsets
                    .iter()
                    .map(|(o, w)| match neighbour(self.shape, at, *o) {
                        Some(q) => w * rho[q],
                        None => w * pad,
                    })
                    .sum();
                sum / self.norms[p]
            })
            .collect())
    }

    /// The filter's transpose applied to `g`: Σᵢ g_i ∂ρ̃ᵢ/∂ρⱼ for each pixel j, the gradient with
    /// respect to the density from the gradient `g` with respect to the filtered density (Wang et
    /// al.'s Eq. 7). With padding the filter is affine, and this is its linear part's transpose.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a gradient that isn't one value per pixel.
    pub fn transpose(&self, g: &[f64]) -> Result<Vec<f64>> {
        self.check_len(g)?;
        // the neighbourhood is symmetric: pixel j's weight in pixel i's average is the weight
        // of i's offset from j, so each column is gathered from j's own neighbours
        Ok((0..g.len())
            .into_par_iter()
            .map(|p| {
                let at = coords(self.shape, p);
                self.offsets
                    .iter()
                    .filter_map(|(o, w)| {
                        neighbour(self.shape, at, *o).map(|q| w * g[q] / self.norms[q])
                    })
                    .sum()
            })
            .collect())
    }
}

/// The threshold projection of Wang et al. 2010's Eq. 9: ρ̂ = (tanh βη + tanh β(ρ̃ − η)) /
/// (tanh βη + tanh β(1 − η)), sharpness β and threshold η.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Projection {
    beta: f64,
    eta: f64,
}

impl Projection {
    /// The projection of sharpness `beta` at threshold `eta`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a β that isn't positive and finite, or an η outside [0, 1].
    pub fn new(beta: f64, eta: f64) -> Result<Projection> {
        if !(beta.is_finite() && beta > 0.0) {
            return Err(Error::invalid(
                "projection",
                format!("β must be positive and finite, got {beta}"),
            ));
        }
        if !(0.0..=1.0).contains(&eta) {
            return Err(Error::invalid(
                "projection",
                format!("η must be in [0, 1], got {eta}"),
            ));
        }
        Ok(Projection { beta, eta })
    }

    /// The sharpness β.
    pub fn beta(&self) -> f64 {
        self.beta
    }

    /// The threshold η.
    pub fn eta(&self) -> f64 {
        self.eta
    }

    fn denominator(&self) -> f64 {
        libm::tanh(self.beta * self.eta) + libm::tanh(self.beta * (1.0 - self.eta))
    }

    /// ρ̂ at the filtered density `x`.
    pub fn value(&self, x: f64) -> f64 {
        (libm::tanh(self.beta * self.eta) + libm::tanh(self.beta * (x - self.eta)))
            / self.denominator()
    }

    /// dρ̂/dρ̃ at `x`: β sech² β(x − η) over the denominator (0 where the cosh overflows).
    pub fn derivative(&self, x: f64) -> f64 {
        let sech = 1.0 / libm::cosh(self.beta * (x - self.eta));
        self.beta * sech * sech / self.denominator()
    }
}

/// A design region's densities, their filter and projection, and the two permittivities they
/// interpolate.
#[derive(Clone, Debug, PartialEq)]
pub struct Density {
    shape: [usize; 3],
    pixel_um: f64,
    eps: [f64; 2],
    filter: Option<ConicFilter>,
    projection: Option<Projection>,
}

impl Density {
    /// A region of `shape` pixels (1 along z for 2D), each `pixel_um` wide, whose permittivity is
    /// `eps[0]` at density 0 and `eps[1]` at density 1; with no filter and no projection until
    /// they are given.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an empty region, a pixel that isn't positive and finite, or a
    /// permittivity that isn't finite.
    pub fn new(shape: [usize; 3], pixel_um: f64, eps: [f64; 2]) -> Result<Density> {
        check_shape(shape, pixel_um)?;
        if !eps.iter().all(|e| e.is_finite()) {
            return Err(Error::invalid(
                "density",
                format!("the permittivities must be finite, got {eps:?}"),
            ));
        }
        Ok(Density {
            shape,
            pixel_um,
            eps,
            filter: None,
            projection: None,
        })
    }

    /// The same region with a [`ConicFilter`] of radius `radius_um`.
    ///
    /// # Errors
    ///
    /// As [`ConicFilter::new`].
    pub fn with_filter(self, radius_um: f64, edge: Edge) -> Result<Density> {
        let filter = ConicFilter::new(self.shape, self.pixel_um, radius_um, edge)?;
        Ok(Density {
            filter: Some(filter),
            ..self
        })
    }

    /// The same region with `projection` after the filter: a continuation stage's β, say.
    pub fn with_projection(self, projection: Projection) -> Density {
        Density {
            projection: Some(projection),
            ..self
        }
    }

    /// The number of pixels.
    pub fn len(&self) -> usize {
        len(self.shape)
    }

    /// Whether the region has no pixels (never: a region has at least one).
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The region's shape in pixels.
    pub fn shape(&self) -> [usize; 3] {
        self.shape
    }

    /// The pixel's width, µm.
    pub fn pixel_um(&self) -> f64 {
        self.pixel_um
    }

    /// The permittivities at density 0 and 1.
    pub fn eps(&self) -> [f64; 2] {
        self.eps
    }

    /// The filter, if any.
    pub fn filter(&self) -> Option<&ConicFilter> {
        self.filter.as_ref()
    }

    /// The projection, if any.
    pub fn projection(&self) -> Option<Projection> {
        self.projection
    }

    fn check_len(&self, v: &[f64], what: &str) -> Result<()> {
        if v.len() != self.len() {
            return Err(Error::invalid(
                "density",
                format!(
                    "expected one {what} per pixel, {}, got {}",
                    self.len(),
                    v.len()
                ),
            ));
        }
        Ok(())
    }

    fn filtered(&self, rho: &[f64]) -> Result<Vec<f64>> {
        self.check_len(rho, "density")?;
        match &self.filter {
            Some(f) => f.apply(rho),
            None => Ok(rho.to_vec()),
        }
    }

    /// The physical density ρ̂ of `rho`: filtered, then projected.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a density that isn't one value per pixel.
    pub fn physical(&self, rho: &[f64]) -> Result<Vec<f64>> {
        let filtered = self.filtered(rho)?;
        Ok(match self.projection {
            Some(p) => filtered.iter().map(|&x| p.value(x)).collect(),
            None => filtered,
        })
    }

    /// The relative permittivity a solver sees at each pixel, ε₀ + ρ̂ (ε₁ − ε₀).
    ///
    /// # Errors
    ///
    /// As [`Density::physical`].
    pub fn permittivity(&self, rho: &[f64]) -> Result<Vec<f64>> {
        let [e0, e1] = self.eps;
        Ok(self
            .physical(rho)?
            .into_iter()
            .map(|x| e0 + x * (e1 - e0))
            .collect())
    }

    /// ∂F/∂ρ from `d_physical`, ∂F/∂ρ̂ at each pixel (FDTD's `Simulation::gradient`, whose
    /// design interpolates the same two permittivities): the projection's derivative at the
    /// filtered density, then the filter's transpose (Wang et al.'s Eq. 7).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a density or gradient that isn't one value per pixel.
    pub fn pullback(&self, rho: &[f64], d_physical: &[f64]) -> Result<Vec<f64>> {
        self.check_len(d_physical, "gradient")?;
        let filtered = self.filtered(rho)?;
        let d_filtered: Vec<f64> = match self.projection {
            Some(p) => filtered
                .iter()
                .zip(d_physical)
                .map(|(&x, &g)| g * p.derivative(x))
                .collect(),
            None => d_physical.to_vec(),
        };
        match &self.filter {
            Some(f) => f.transpose(&d_filtered),
            None => Ok(d_filtered),
        }
    }

    /// ∂F/∂ρ from `d_eps`, ∂F/∂ε at each pixel (2D FDFD's `mode_power_gradient`, restricted to
    /// the region by [`Placement::restrict`]): [`Density::pullback`] of (ε₁ − ε₀) ∂F/∂ε.
    ///
    /// # Errors
    ///
    /// As [`Density::pullback`].
    pub fn gradient(&self, rho: &[f64], d_eps: &[f64]) -> Result<Vec<f64>> {
        let scale = self.eps[1] - self.eps[0];
        let d_physical: Vec<f64> = d_eps.iter().map(|g| g * scale).collect();
        self.pullback(rho, &d_physical)
    }
}

/// Where a region's pixels sit on a solver's cells, one pixel a cell: the region's first cell
/// and the solver's grid in cells, both ordered i fastest, then j, then k (2D FDFD's cell
/// `[j * nx + i]` with 1 along z).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    /// The cell of the region's pixel (0, 0, 0).
    pub origin: [usize; 3],
    /// The region's shape in pixels.
    pub shape: [usize; 3],
    /// The solver's grid in cells.
    pub grid: [usize; 3],
}

impl Placement {
    fn check(&self, region: usize, cells: usize) -> Result<()> {
        let fits = (0..3).all(|a| self.origin[a] + self.shape[a] <= self.grid[a]);
        if !fits || len(self.shape) == 0 {
            return Err(Error::invalid(
                "placement",
                format!(
                    "a region of {:?} pixels from cell {:?} doesn't fit a grid of {:?}",
                    self.shape, self.origin, self.grid
                ),
            ));
        }
        if region != len(self.shape) || cells != len(self.grid) {
            return Err(Error::invalid(
                "placement",
                format!(
                    "expected {} region values and {} cells, got {region} and {cells}",
                    len(self.shape),
                    len(self.grid)
                ),
            ));
        }
        Ok(())
    }

    fn cell(&self, p: usize) -> usize {
        let [i, j, k] = coords(self.shape, p);
        let [i0, j0, k0] = self.origin;
        ((k + k0) * self.grid[1] + (j + j0)) * self.grid[0] + (i + i0)
    }

    /// `values`, one per pixel, written into the region's cells of `cells`, one per cell.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a region that doesn't fit the grid, or the wrong number of
    /// values or cells.
    pub fn embed<T: From<f64>>(&self, values: &[f64], cells: &mut [T]) -> Result<()> {
        self.check(values.len(), cells.len())?;
        for (p, &v) in values.iter().enumerate() {
            cells[self.cell(p)] = T::from(v);
        }
        Ok(())
    }

    /// The region's values of `cells`, one per cell: a solver's gradient restricted to the
    /// region's pixels.
    ///
    /// # Errors
    ///
    /// As [`Placement::embed`].
    pub fn restrict(&self, cells: &[f64]) -> Result<Vec<f64>> {
        self.check(len(self.shape), cells.len())?;
        Ok((0..len(self.shape)).map(|p| cells[self.cell(p)]).collect())
    }
}
