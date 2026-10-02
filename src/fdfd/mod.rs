//! Frequency-domain finite differences (FDFD): Maxwell's equations at one frequency, on a grid,
//! as one sparse linear system.
//!
//! **2D** ([`Solver2d`]): the structure is invariant along z and the fields vary in the x–y
//! plane, so they split into two polarizations (K. S. Yee, IEEE Trans. Antennas Propag. 14, 302
//! (1966), doi:10.1109/TAP.1966.1138693, "Maxwell's equations in two dimensions"):
//!
//! - [`Polarization::Ez`]: E along z (with H_x, H_y): ∇²E_z + k₀² ε E_z = −i k₀ J, J = η₀ J_z;
//! - [`Polarization::Hz`]: H along z (with E_x, E_y):
//!   ∂_x(ε_y⁻¹ ∂_x H̃_z) + ∂_y(ε_x⁻¹ ∂_y H̃_z) + k₀² H̃_z = −i k₀ M, M the magnetic current,
//!
//! with H in units of E/η₀ (H̃ = η₀ H) and e^(−iωt) throughout. On Yee's staggered grid the
//! field along z sits at the cells' centres and the in-plane components on the faces between
//! them (his Fig. 1, in 2D), so every derivative is a centred difference: second order.
//!
//! **Open boundaries** are stretched-coordinate PMLs (W. C. Chew, W. H. Weedon, Microw. Opt.
//! Technol. Lett. 7, 599 (1994), doi:10.1002/mop.4650071304): ∂_w → s_w⁻¹ ∂_w, with
//! s_w = 1 + iσ_w/(ωε₀) graded as σ_w = σ_max (l/d)^m over a layer d thick and
//! σ_max = −(m + 1) ln R / (2η₀ d) for a target normal-incidence reflection R (W. Shin, S. Fan,
//! J. Comput. Phys. 231, 3406 (2012), doi:10.1016/j.jcp.2012.01.013, Eqs. 2.5–2.9; their
//! e^(+iωt) makes their s 1 − iσ/(ωε₀)). In units of k₀, s = 1 + i (m + 1)(−ln R)/(2k₀d) (l/d)^m.
//! Shin and Fan show that this PML, unlike the uniaxial one, keeps the system well conditioned.
//! An axis can instead be **Bloch-periodic**: one period on, the field is e^(ikL) times itself.
//!
//! **Interfaces:** each field component sees the permittivity averaged over its own cell of the
//! grid, harmonically along the component and arithmetically across it, which is what a
//! layered medium is for a field along or across its layers.

use faer::sparse::linalg::solvers::Lu;
use faer::sparse::{SparseColMat, Triplet};
use num_complex::Complex64 as c64;

use crate::units::Wavelength;
use crate::{Error, Result};

/// The field along z, which fixes the polarization of a 2D problem.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Polarization {
    /// E along z, H in the plane: TE for layers normal to the plane's y axis.
    Ez,
    /// H along z, E in the plane: TM for such layers.
    Hz,
}

/// A uniform grid of `nx` × `ny` cells of `dx` × `dy` µm, its lower left corner at (`x0`, `y0`).
/// The unknowns, the field along z, sit at the cells' centres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    /// Cells along x.
    pub nx: usize,
    /// Cells along y.
    pub ny: usize,
    /// Cell width, µm.
    pub dx: f64,
    /// Cell height, µm.
    pub dy: f64,
    /// The grid's left edge, µm.
    pub x0: f64,
    /// Its bottom edge, µm.
    pub y0: f64,
}

impl Grid {
    /// The x of column `i`'s centre, µm.
    pub fn x(&self, i: usize) -> f64 {
        self.x0 + (i as f64 + 0.5) * self.dx
    }

    /// The y of row `j`'s centre, µm.
    pub fn y(&self, j: usize) -> f64 {
        self.y0 + (j as f64 + 0.5) * self.dy
    }

    fn check(&self) -> Result<()> {
        let ok = |v: f64| v.is_finite() && v > 0.0;
        if self.nx == 0 || self.ny == 0 || !ok(self.dx) || !ok(self.dy) {
            return Err(Error::invalid(
                "fdfd grid",
                format!(
                    "needs cells and positive steps, got {} x {} cells of {} x {} um",
                    self.nx, self.ny, self.dx, self.dy
                ),
            ));
        }
        if !(self.x0.is_finite() && self.y0.is_finite()) {
            return Err(Error::invalid("fdfd grid", "its corner must be finite"));
        }
        Ok(())
    }
}

/// What lies beyond one axis's two ends.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Edges {
    /// A PML `low` cells thick at the low end and `high` at the high end, inside the grid, and
    /// the field along z zero beyond. Zero cells is a plain wall.
    Pml {
        /// Cells of PML at the low end.
        low: usize,
        /// Cells of PML at the high end.
        high: usize,
    },
    /// Bloch-periodic: one period (the grid's length along this axis) on, the field is
    /// e^(ik × period) times itself, `k` in rad/µm.
    Bloch {
        /// The Bloch wavenumber, rad/µm.
        k: f64,
    },
}

impl Edges {
    fn pml(self) -> (usize, usize) {
        match self {
            Edges::Pml { low, high } => (low, high),
            Edges::Bloch { .. } => (0, 0),
        }
    }
}

/// The grid's boundaries, and the PMLs' grading.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Boundaries {
    /// Along x.
    pub x: Edges,
    /// Along y.
    pub y: Edges,
    /// The PMLs' target reflection at normal incidence, R in (0, 1).
    pub reflection: f64,
    /// The polynomial grading's order m.
    pub order: f64,
}

impl Boundaries {
    /// PMLs `cells` thick on all four sides, graded to R = 1e-8 with m = 3.
    pub fn pml(cells: usize) -> Boundaries {
        let edges = Edges::Pml {
            low: cells,
            high: cells,
        };
        Boundaries {
            x: edges,
            y: edges,
            reflection: 1e-8,
            order: 3.0,
        }
    }
}

/// The permittivity of the cell [x ± hx/2] × [y ± hy/2] for a field component along `along`
/// (0: x, 1: y, 2: z), from `samples`² points: harmonic along the component and arithmetic
/// across it; along z, arithmetic over the cell.
fn averaged(
    eps: &impl Fn(f64, f64) -> c64,
    (x, y): (f64, f64),
    (hx, hy): (f64, f64),
    along: usize,
    samples: usize,
) -> c64 {
    let at = |a: usize, b: usize| {
        let u = (a as f64 + 0.5) / samples as f64 - 0.5;
        let v = (b as f64 + 0.5) / samples as f64 - 0.5;
        eps(x + u * hx, y + v * hy)
    };
    let n = samples as f64;
    let one = c64::new(1.0, 0.0);
    let harmonic = |line: &dyn Fn(usize) -> c64| one / ((0..samples).map(line).sum::<c64>() / n);
    match along {
        0 => {
            (0..samples)
                .map(|b| harmonic(&|a| one / at(a, b)))
                .sum::<c64>()
                / n
        }
        1 => {
            (0..samples)
                .map(|a| harmonic(&|b| one / at(a, b)))
                .sum::<c64>()
                / n
        }
        _ => {
            (0..samples)
                .flat_map(|a| (0..samples).map(move |b| (a, b)))
                .map(|(a, b)| at(a, b))
                .sum::<c64>()
                / (n * n)
        }
    }
}

/// The PML stretch at `pos` on an axis spanning `span`, with `layers` cells of PML of width `h`
/// at its ends.
fn stretch(
    pos: f64,
    span: (f64, f64),
    layers: (usize, usize),
    h: f64,
    k0: f64,
    b: &Boundaries,
) -> c64 {
    let profile = |depth: f64, d: f64| {
        let sigma = (b.order + 1.0) * (-b.reflection.ln()) / (2.0 * k0 * d);
        c64::new(1.0, sigma * (depth / d).clamp(0.0, 1.0).powf(b.order))
    };
    let (dl, dh) = (layers.0 as f64 * h, layers.1 as f64 * h);
    if dl > 0.0 && pos < span.0 + dl {
        profile(span.0 + dl - pos, dl)
    } else if dh > 0.0 && pos > span.1 - dh {
        profile(pos - (span.1 - dh), dh)
    } else {
        c64::new(1.0, 0.0)
    }
}

/// A 2D FDFD problem, assembled and factorized: one structure at one wavelength, ready for any
/// number of sources.
pub struct Solver2d {
    grid: Grid,
    polarization: Polarization,
    k0: f64,
    boundaries: Boundaries,
    /// Ez: ε_z at the centres, `nx` × `ny`.
    eps_z: Vec<c64>,
    /// Hz: ε_y on the x-faces (x0 + i dx, i = 0 … nx), `(nx + 1)` × `ny`.
    eps_y: Vec<c64>,
    /// Hz: ε_x on the y-faces (y0 + j dy, j = 0 … ny), `nx` × `(ny + 1)`.
    eps_x: Vec<c64>,
    /// The matrix's entries, for its products with sources.
    entries: Vec<Triplet<usize, usize, c64>>,
    lu: Lu<usize, c64>,
}

impl Solver2d {
    /// Assembles and factorizes `polarization` at `wavelength` on `grid`, with relative
    /// permittivity `eps(x, y)` (µm), inside `boundaries`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an empty grid, PMLs that fill an axis, a target reflection
    /// outside (0, 1), a negative order or a non-finite Bloch wavenumber, or a system that can't
    /// be factorized.
    pub fn new(
        grid: Grid,
        polarization: Polarization,
        wavelength: Wavelength,
        eps: impl Fn(f64, f64) -> c64,
        boundaries: Boundaries,
    ) -> Result<Solver2d> {
        grid.check()?;
        let b = boundaries;
        if !(b.reflection > 0.0 && b.reflection < 1.0) || !(b.order.is_finite() && b.order >= 0.0) {
            return Err(Error::invalid(
                "fdfd boundaries",
                format!(
                    "the PML needs a target reflection in (0, 1) and an order >= 0, got {} and {}",
                    b.reflection, b.order
                ),
            ));
        }
        for (edges, n) in [(b.x, grid.nx), (b.y, grid.ny)] {
            match edges {
                Edges::Pml { low, high } if low + high >= n => {
                    return Err(Error::invalid(
                        "fdfd boundaries",
                        format!("PMLs of {low} and {high} cells leave nothing of {n} cells"),
                    ));
                }
                Edges::Bloch { k } if !k.is_finite() => {
                    return Err(Error::invalid(
                        "fdfd boundaries",
                        "the Bloch wavenumber must be finite",
                    ));
                }
                _ => {}
            }
        }
        let k0 = 2.0 * std::f64::consts::PI / wavelength.to_um();
        let (nx, ny, h) = (grid.nx, grid.ny, (grid.dx, grid.dy));
        let samples = 8;
        let (mut eps_z, mut eps_y, mut eps_x) = (Vec::new(), Vec::new(), Vec::new());
        match polarization {
            Polarization::Ez => {
                for j in 0..ny {
                    for i in 0..nx {
                        eps_z.push(averaged(&eps, (grid.x(i), grid.y(j)), h, 2, samples));
                    }
                }
            }
            Polarization::Hz => {
                for j in 0..ny {
                    for i in 0..=nx {
                        let x = grid.x0 + i as f64 * grid.dx;
                        eps_y.push(averaged(&eps, (x, grid.y(j)), h, 1, samples));
                    }
                }
                for j in 0..=ny {
                    for i in 0..nx {
                        let y = grid.y0 + j as f64 * grid.dy;
                        eps_x.push(averaged(&eps, (grid.x(i), y), h, 0, samples));
                    }
                }
            }
        }
        let entries = assemble(grid, polarization, k0, &eps_z, &eps_y, &eps_x, &b);
        let lu = factorize(&entries, nx * ny)?;
        Ok(Solver2d {
            grid,
            polarization,
            k0,
            boundaries,
            eps_z,
            eps_y,
            eps_x,
            entries,
            lu,
        })
    }

    /// The grid.
    pub fn grid(&self) -> Grid {
        self.grid
    }

    /// The field for a source: J = η₀ J_z for Ez, or the magnetic current M_z for Hz, one value
    /// per cell, row by row from the bottom.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the source isn't one value per cell.
    pub fn solve(&self, source: &[c64]) -> Result<Field2d> {
        let rhs: Vec<c64> = source.iter().map(|s| c64::new(0.0, -self.k0) * s).collect();
        self.solve_system(&rhs)
    }

    /// The field u of A u = `rhs`, A the assembled matrix: for sources already in its form,
    /// such as [`Solver2d::mode_source`]'s.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if `rhs` isn't one value per cell.
    pub fn solve_system(&self, rhs: &[c64]) -> Result<Field2d> {
        use faer::linalg::solvers::Solve;
        let n = self.grid.nx * self.grid.ny;
        if rhs.len() != n {
            return Err(Error::invalid(
                "fdfd source",
                format!("needs {n} values, one per cell, got {}", rhs.len()),
            ));
        }
        let b = faer::Mat::<c64>::from_fn(n, 1, |r, _| rhs[r]);
        let u = self.lu.solve(&b);
        let mut values: Vec<c64> = (0..n).map(|r| u[(r, 0)]).collect();
        // one step of iterative refinement: the factorization's rounding, PMLs and strong
        // contrasts make it worth it
        let residual = self.apply(&values);
        let r = faer::Mat::<c64>::from_fn(n, 1, |k, _| rhs[k] - residual[k]);
        let correction = self.lu.solve(&r);
        for (k, v) in values.iter_mut().enumerate() {
            *v += correction[(k, 0)];
        }
        Ok(Field2d {
            grid: self.grid,
            polarization: self.polarization,
            k0: self.k0,
            values,
            eps_x: self.eps_x.clone(),
            eps_y: self.eps_y.clone(),
        })
    }

    /// A v, the assembled matrix times `v`.
    fn apply(&self, v: &[c64]) -> Vec<c64> {
        let mut out = vec![c64::new(0.0, 0.0); v.len()];
        for t in &self.entries {
            out[t.row] += t.val * v[t.col];
        }
        out
    }
}

/// The matrix: at each cell, the four faces' couplings and the diagonal.
fn assemble(
    g: Grid,
    polarization: Polarization,
    k0: f64,
    eps_z: &[c64],
    eps_y: &[c64],
    eps_x: &[c64],
    b: &Boundaries,
) -> Vec<Triplet<usize, usize, c64>> {
    let (nx, ny) = (g.nx, g.ny);
    let one = c64::new(1.0, 0.0);
    let span_x = (g.x0, g.x0 + nx as f64 * g.dx);
    let span_y = (g.y0, g.y0 + ny as f64 * g.dy);
    let sx = |x: f64| stretch(x, span_x, b.x.pml(), g.dx, k0, b);
    let sy = |y: f64| stretch(y, span_y, b.y.pml(), g.dy, k0, b);
    // the face's weight: 1 for Ez, 1/ε of the E component crossing it for Hz
    let weight = |eps: c64| match polarization {
        Polarization::Ez => one,
        Polarization::Hz => one / eps,
    };
    // across the end of an axis: the neighbour's index along it and its phase, if any
    let beyond = |n: i64, len: usize, edges: Edges, step: f64| -> Option<(usize, c64)> {
        if (0..len as i64).contains(&n) {
            return Some((n as usize, one));
        }
        match edges {
            Edges::Bloch { k } => {
                let turns = if n < 0 { -1.0 } else { 1.0 };
                Some((
                    n.rem_euclid(len as i64) as usize,
                    c64::from_polar(1.0, turns * k * len as f64 * step),
                ))
            }
            Edges::Pml { .. } => None,
        }
    };
    let mut t = Vec::with_capacity(5 * nx * ny);
    for j in 0..ny {
        for i in 0..nx {
            let k = j * nx + i;
            let (x, y) = (g.x(i), g.y(j));
            let mut diagonal = match polarization {
                Polarization::Ez => k0 * k0 * eps_z[k],
                Polarization::Hz => c64::new(k0 * k0, 0.0),
            };
            let (ey_left, ey_right) = match polarization {
                Polarization::Ez => (one, one),
                Polarization::Hz => (eps_y[j * (nx + 1) + i], eps_y[j * (nx + 1) + i + 1]),
            };
            let (ex_bottom, ex_top) = match polarization {
                Polarization::Ez => (one, one),
                Polarization::Hz => (eps_x[j * nx + i], eps_x[(j + 1) * nx + i]),
            };
            // (step along x, along y, the face's stretch, the step, the face's weight)
            let faces = [
                (
                    -1i64,
                    0i64,
                    sx(x) * sx(x - g.dx / 2.0),
                    g.dx,
                    weight(ey_left),
                ),
                (1, 0, sx(x) * sx(x + g.dx / 2.0), g.dx, weight(ey_right)),
                (0, -1, sy(y) * sy(y - g.dy / 2.0), g.dy, weight(ex_bottom)),
                (0, 1, sy(y) * sy(y + g.dy / 2.0), g.dy, weight(ex_top)),
            ];
            for (di, dj, s, step, w) in faces {
                let c = w / (s * step * step);
                diagonal -= c;
                let target = if di != 0 {
                    beyond(i as i64 + di, nx, b.x, g.dx).map(|(n, p)| (j * nx + n, p))
                } else {
                    beyond(j as i64 + dj, ny, b.y, g.dy).map(|(n, p)| (n * nx + i, p))
                };
                if let Some((col, phase)) = target {
                    t.push(Triplet::new(k, col, c * phase));
                }
            }
            t.push(Triplet::new(k, k, diagonal));
        }
    }
    t
}

fn factorize(triplets: &[Triplet<usize, usize, c64>], n: usize) -> Result<Lu<usize, c64>> {
    let numerical = |reason: String| Error::invalid("fdfd", reason);
    let matrix = SparseColMat::<usize, c64>::try_new_from_triplets(n, n, triplets)
        .map_err(|e| numerical(format!("can't assemble the matrix: {e:?}")))?;
    matrix
        .sp_lu()
        .map_err(|e| numerical(format!("can't factorize the matrix: {e:?}")))
}

/// A 2D FDFD solution: the field along z at the cells' centres.
#[derive(Clone, Debug, PartialEq)]
pub struct Field2d {
    grid: Grid,
    polarization: Polarization,
    k0: f64,
    values: Vec<c64>,
    /// Hz: ε_x on the y-faces and ε_y on the x-faces, for the fluxes.
    eps_x: Vec<c64>,
    eps_y: Vec<c64>,
}

impl Field2d {
    /// The field along z (E_z, or H̃_z = η₀ H_z) at cell (i, j)'s centre.
    pub fn at(&self, i: usize, j: usize) -> c64 {
        self.values[j * self.grid.nx + i]
    }

    /// This field minus `other`, a field of the same problem: the scattered field, given the
    /// incident one.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if they aren't on the same grid with the same polarization.
    pub fn minus(&self, other: &Field2d) -> Result<Field2d> {
        if self.grid != other.grid || self.polarization != other.polarization {
            return Err(Error::invalid(
                "fdfd field",
                "can only subtract a field on the same grid with the same polarization",
            ));
        }
        Ok(Field2d {
            values: self
                .values
                .iter()
                .zip(&other.values)
                .map(|(a, b)| a - b)
                .collect(),
            ..self.clone()
        })
    }

    /// The power crossing the face between rows `j` and `j + 1` upwards (+y), per unit length
    /// along z: the row's sum of ½ Re(E × H̃*)_y dx (so η₀ = 1; in |field|² µm). This is the
    /// scheme's own flux: in a lossless region without sources it is exactly the same through
    /// every row.
    ///
    /// # Panics
    ///
    /// If row `j + 1` isn't on the grid.
    pub fn flux_y(&self, j: usize) -> f64 {
        let g = self.grid;
        assert!(
            j + 1 < g.ny,
            "flux_y needs rows {j} and {} on the grid",
            j + 1
        );
        let i_k0 = c64::new(0.0, self.k0);
        (0..g.nx)
            .map(|i| {
                let (a, b) = (self.at(i, j), self.at(i, j + 1));
                let (mean, slope) = (0.5 * (a + b), (b - a) / g.dy);
                let s = match self.polarization {
                    // S_y = ½ Re(E_z H̃_x*), with H̃_x = ∂_y E_z / (i k0)
                    Polarization::Ez => 0.5 * (mean * (slope / i_k0).conj()).re,
                    // S_y = −½ Re(E_x H̃_z*), with E_x = −∂_y H̃_z / (i k0 ε_x)
                    Polarization::Hz => {
                        let ex = -slope / (i_k0 * self.eps_x[(j + 1) * g.nx + i]);
                        -0.5 * (ex * mean.conj()).re
                    }
                };
                s * g.dx
            })
            .sum()
    }
}

pub(crate) mod checks;
mod ports;
#[cfg(test)]
mod tests;

pub use ports::{Direction, Port, PortMode, Side};
