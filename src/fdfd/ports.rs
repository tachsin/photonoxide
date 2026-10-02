//! Ports: waveguide modes in and out of a 2D FDFD problem, and its S-parameters.
//!
//! A port is a column of the grid that crosses a waveguide running along x. Its **modes** are
//! the scheme's own: the eigenvectors of the 2D operator restricted to the column, with the
//! field along x going as e^(iβx) on the grid. For E along z,
//! L_y u + k₀² ε_z u = β_d² u, and for H along z, ε_y (L_y + k₀²) u = β_d² u, where L_y is the
//! column's y-part of the operator (PML included) and β_d = (2/Δx) sin(βΔx/2) is what the grid's
//! difference along x makes of β. A mode solved this way propagates along a straight grid
//! waveguide exactly: launched, it neither reflects nor sheds radiation.
//!
//! **Sources** are total-field/scattered-field (R. C. Rumpf, Prog. Electromagn. Res. B 36, 221
//! (2012), doi:10.2528/PIERB11092006, Eq. 55): with Q masking the scattered-field cells and
//! f the mode extended along x as e^(±iβx), b = (QA − AQ) f. The mode then travels one way
//! only, from the interface into the total-field side, and the scattered-field side holds only
//! what the device sends back.
//!
//! **Amplitudes:** on two neighbouring columns c and c + 1, the field's share of a mode is a
//! projection with the operator's own orthogonality, Σ_j w_j φ_m φ_n = 0 for m ≠ n, where
//! w = s_y (the PML stretch) for E along z and s_y/ε_y for H along z, the weights that make the
//! column's operator symmetric. Each mode normalized to Σ w φ² = 1 is real where the guide is
//! lossless. The two columns' shares p_c = a + b and p_(c+1) = a e^(iβΔx) + b e^(−iβΔx) give
//! the forward and backward amplitudes a and b at column c.
//!
//! **S-parameters** are power-normalized: S_qp = (outgoing at q) √P_q / ((incoming at p) √P_p),
//! with P a mode's power at unit amplitude, so |S_qp|² is the share of power from mode p into
//! mode q. P is taken in its unconjugated form, the scheme's Lorentz form between the mode going
//! forward and backward, which is the power for a real mode and makes S exactly symmetric for a
//! reciprocal device.

use num_complex::Complex64 as c64;

use super::{Edges, Field2d, Polarization, Solver2d, stretch};
use crate::{Error, Result};

/// Which way along x a wave travels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Towards +x.
    Forward,
    /// Towards −x.
    Backward,
}

/// A mode of a port: a waveguide's cross-section along one column of the grid.
#[derive(Clone, Debug, PartialEq)]
pub struct PortMode {
    column: usize,
    k0: f64,
    dx: f64,
    beta: c64,
    /// The profile along the column, one value per row, normalized to Σ w φ² = 1.
    profile: Vec<c64>,
    /// The projection's weights w.
    weights: Vec<c64>,
    /// For H along z, ε_y on the face between the column and the next; 1 for E along z.
    eps_face: Vec<c64>,
    polarization: Polarization,
}

impl PortMode {
    /// The column the mode was solved on; its amplitudes are measured on it and the next.
    pub fn column(&self) -> usize {
        self.column
    }

    /// The propagation constant β, rad/µm.
    pub fn beta(&self) -> c64 {
        self.beta
    }

    /// The effective index, β/k₀.
    pub fn effective_index(&self) -> c64 {
        self.beta / self.k0
    }

    /// The field along z down the column, row by row, normalized to Σ w φ² = 1.
    pub fn profile(&self) -> &[c64] {
        &self.profile
    }

    /// The power the mode carries forward at unit amplitude, per unit length along z, η₀ = 1:
    /// the scheme's flux between the column and the next.
    pub fn power(&self, dy: f64) -> f64 {
        let step = (c64::new(0.0, 1.0) * self.beta * self.dx).exp();
        self.profile
            .iter()
            .zip(&self.eps_face)
            .map(|(&a, &eps)| flux(self.polarization, self.k0, self.dx, a, a * step, eps) * dy)
            .sum()
    }

    /// The unconjugated version of [`PortMode::power`], sin(βΔx) Δy / (2k₀Δx), which discrete
    /// reciprocity normalizes by: the scheme's Lorentz form between the mode going forward and
    /// going backward, given Σ w φ² = 1. It is the power when the mode is real, a lossless guide's
    /// clear of the PMLs; where its tail reaches into a PML the two differ, by about the share of
    /// its power there.
    fn reciprocal_power(&self, dy: f64) -> c64 {
        (self.beta * self.dx).sin() * dy / (2.0 * self.k0 * self.dx)
    }

    /// The share of a column's field `v` in this mode: Σ w φ v (Σ w φ² = 1).
    fn share(&self, v: impl Iterator<Item = c64>) -> c64 {
        self.weights
            .iter()
            .zip(&self.profile)
            .zip(v)
            .map(|((w, p), v)| w * p * v)
            .sum()
    }
}

/// The power flux density along x between two neighbouring cells a and b, Δx apart:
/// ½ Re(E × H̃*)_x, the in-plane component and the field along z taken on the face between them.
fn flux(polarization: Polarization, k0: f64, dx: f64, a: c64, b: c64, eps_face: c64) -> f64 {
    let i_k0 = c64::new(0.0, k0);
    let (mean, slope) = (0.5 * (a + b), (b - a) / dx);
    match polarization {
        // S_x = −½ Re(E_z H̃_y*), with H̃_y = −∂_x E_z / (i k0)
        Polarization::Ez => 0.5 * (mean * (slope / i_k0).conj()).re,
        // S_x = ½ Re(E_y H̃_z*), with E_y = ∂_x H̃_z / (i k0 ε_y)
        Polarization::Hz => 0.5 * (slope / (i_k0 * eps_face) * mean.conj()).re,
    }
}

/// Which side of the device a port is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    /// On the left: light comes in towards +x and leaves towards −x.
    Left,
    /// On the right: light comes in towards −x and leaves towards +x.
    Right,
}

/// A port of a device: a mode, and the side it is on.
#[derive(Clone, Debug, PartialEq)]
pub struct Port {
    /// The mode in and out.
    pub mode: PortMode,
    /// Its side.
    pub side: Side,
}

impl Port {
    fn incoming(&self) -> Direction {
        match self.side {
            Side::Left => Direction::Forward,
            Side::Right => Direction::Backward,
        }
    }
}

impl Solver2d {
    /// The `count` modes of the waveguide crossing column `column`, highest effective index
    /// first, as the scheme sees them (see the module's docs). The column and the next must be
    /// outside the PMLs along x, with the guide the same on both.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the column isn't at least two cells clear of a PML or the
    /// grid's end along x, if y is Bloch-periodic, or if the modes don't converge.
    pub fn port_modes(&self, column: usize, count: usize) -> Result<Vec<PortMode>> {
        self.port_modes_within(column, 0..self.grid.ny, count)
    }

    /// The modes of a port that spans only `rows` of the column: one guide of several side by
    /// side, each its own port. The modes are solved with walls at the window's ends and are zero
    /// outside it, and the projection sees only the window, so the guide's field must have
    /// decayed to nothing at its ends; otherwise as [`Solver2d::port_modes`].
    ///
    /// # Errors
    ///
    /// As [`Solver2d::port_modes`], and [`Error::InvalidValue`] for a window of fewer than 3 rows
    /// or past the grid.
    pub fn port_modes_within(
        &self,
        column: usize,
        rows: std::ops::Range<usize>,
        count: usize,
    ) -> Result<Vec<PortMode>> {
        let g = self.grid;
        if rows.end > g.ny || rows.len() < 3 {
            return Err(Error::invalid(
                "fdfd port",
                format!(
                    "the window, rows {} to {}, must have 3 rows or more on the grid's {}",
                    rows.start, rows.end, g.ny
                ),
            ));
        }
        let b = &self.boundaries;
        if matches!(b.y, Edges::Bloch { .. }) {
            return Err(Error::invalid(
                "fdfd port",
                "a port needs PMLs or walls along y, not Bloch-periodic sides",
            ));
        }
        let (low, high) = b.x.pml();
        let clear = column >= low + 2 && column + 3 + high <= g.nx;
        if !clear {
            return Err(Error::invalid(
                "fdfd port",
                format!(
                    "column {column} must be two cells clear of the PMLs ({low} and {high} cells) and the ends of {} columns",
                    g.nx
                ),
            ));
        }
        let (ny, k0) = (g.ny, self.k0);
        let span_y = (g.y0, g.y0 + ny as f64 * g.dy);
        let sy = |y: f64| stretch(y, span_y, b.y.pml(), g.dy, k0, b);
        let one = c64::new(1.0, 0.0);
        // ε_y on the face between the column and the next (H along z)
        let eps_face: Vec<c64> = (0..ny)
            .map(|j| match self.polarization {
                Polarization::Ez => one,
                Polarization::Hz => self.eps_y[j * (g.nx + 1) + column + 1],
            })
            .collect();
        let (start, m) = (rows.start, rows.len());
        let mut entries = Vec::with_capacity(3 * m);
        let mut shift_eps: f64 = 1.0;
        for (j, &face_eps) in eps_face.iter().enumerate().skip(start).take(m) {
            let y = g.y(j);
            let (scale, mut diagonal) = match self.polarization {
                Polarization::Ez => {
                    let eps = self.eps_z[j * g.nx + column];
                    shift_eps = shift_eps.max(eps.re);
                    (one, k0 * k0 * eps)
                }
                Polarization::Hz => {
                    shift_eps = shift_eps.max(face_eps.re);
                    (face_eps, c64::new(k0 * k0, 0.0))
                }
            };
            for (dj, face) in [(-1i64, y - g.dy / 2.0), (1, y + g.dy / 2.0)] {
                let weight = match self.polarization {
                    Polarization::Ez => one,
                    Polarization::Hz => {
                        let row = if dj < 0 { j } else { j + 1 };
                        one / self.eps_x[row * g.nx + column]
                    }
                };
                let c = weight / (sy(y) * sy(face) * g.dy * g.dy);
                diagonal -= c;
                let n = j as i64 + dj;
                if rows.contains(&(n.max(0) as usize)) && n >= 0 {
                    entries.push((j - start, n as usize - start, scale * c));
                }
            }
            entries.push((j - start, j - start, scale * diagonal));
        }
        let shift = c64::new(k0 * k0 * shift_eps, 0.0);
        let pairs = crate::eigen::nearest(m, &entries, shift, count, 1e-10)?;
        // the projection sees only the window
        let weights: Vec<c64> = (0..ny)
            .map(|j| {
                if rows.contains(&j) {
                    sy(g.y(j)) / eps_face[j]
                } else {
                    c64::new(0.0, 0.0)
                }
            })
            .collect();
        let mut modes: Vec<PortMode> = pairs
            .into_iter()
            .map(|p| {
                // β from β_d: the grid's difference along x
                let beta_d = p.value.sqrt();
                let beta_d = if beta_d.re < 0.0 { -beta_d } else { beta_d };
                let beta = 2.0 / g.dx * (beta_d * g.dx / 2.0).asin();
                // the window's values in the column, zero outside it
                let mut profile = vec![c64::new(0.0, 0.0); ny];
                profile[start..start + m].copy_from_slice(&p.vector);
                // Σ w φ² = 1, which makes a lossless guide's mode real
                let norm: c64 = weights.iter().zip(&profile).map(|(w, v)| w * v * v).sum();
                let scale = one / norm.sqrt();
                profile.iter_mut().for_each(|v| *v *= scale);
                // the sign: the largest value positive
                let peak = profile
                    .iter()
                    .copied()
                    .max_by(|a, b| a.norm().total_cmp(&b.norm()))
                    .unwrap_or(one);
                if peak.re < 0.0 {
                    profile.iter_mut().for_each(|v| *v = -*v);
                }
                PortMode {
                    column,
                    k0,
                    dx: g.dx,
                    beta,
                    profile,
                    weights: weights.clone(),
                    eps_face: eps_face.clone(),
                    polarization: self.polarization,
                }
            })
            .collect();
        modes.sort_by(|a, b| b.beta.re.total_cmp(&a.beta.re));
        Ok(modes)
    }

    /// The right-hand side that launches `mode` at unit amplitude travelling `direction`, by
    /// total-field/scattered-field. Going forward, the total field is the columns from the
    /// mode's onwards; going backward, those up to the one after it. Either way the mode's
    /// column and the next are in the total field, where its amplitude is 1 at the mode's column.
    /// The guide must be the same along x around those columns.
    pub fn mode_source(&self, mode: &PortMode, direction: Direction) -> Vec<c64> {
        let g = self.grid;
        let c = mode.column as i64;
        let sign = match direction {
            Direction::Forward => 1.0,
            Direction::Backward => -1.0,
        };
        let i_beta = c64::new(0.0, sign) * mode.beta;
        let incident: Vec<c64> = (0..g.ny)
            .flat_map(|j| {
                (0..g.nx)
                    .map(move |i| mode.profile[j] * (i_beta * ((i as i64 - c) as f64 * g.dx)).exp())
            })
            .collect();
        let scattered = |i: usize| match direction {
            Direction::Forward => (i as i64) < c,
            Direction::Backward => (i as i64) > c + 1,
        };
        let masked: Vec<c64> = incident
            .iter()
            .enumerate()
            .map(|(k, &f)| {
                if scattered(k % g.nx) {
                    f
                } else {
                    c64::new(0.0, 0.0)
                }
            })
            .collect();
        let a_f = self.apply(&incident);
        let a_qf = self.apply(&masked);
        // b = (QA − AQ) f
        (0..g.nx * g.ny)
            .map(|k| {
                let q = if scattered(k % g.nx) {
                    a_f[k]
                } else {
                    c64::new(0.0, 0.0)
                };
                q - a_qf[k]
            })
            .collect()
    }

    /// The power-normalized S-matrix of the device between `ports`: `s[q][p]`, from mode p going
    /// in to mode q coming out (see the module's docs).
    ///
    /// One solve per port, each launching that port's mode. Every run's incoming and outgoing
    /// amplitudes are measured at every port, and S solves S A = B, A the incoming and B the
    /// outgoing ones, a column per run. What the PMLs send back into a port, in its mode, is then
    /// part of A instead of an error in S.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if a port's modes don't belong to this problem.
    pub fn s_matrix(&self, ports: &[Port]) -> Result<Vec<Vec<c64>>> {
        use faer::linalg::solvers::Solve;
        for p in ports {
            if p.mode.profile.len() != self.grid.ny || p.mode.polarization != self.polarization {
                return Err(Error::invalid(
                    "fdfd port",
                    "a port's mode must come from this problem's port_modes",
                ));
            }
        }
        let n = ports.len();
        let roots: Vec<c64> = ports
            .iter()
            .map(|p| p.mode.reciprocal_power(self.grid.dy).sqrt())
            .collect();
        let mut incoming = faer::Mat::<c64>::zeros(n, n);
        let mut outgoing = faer::Mat::<c64>::zeros(n, n);
        for (p, port) in ports.iter().enumerate() {
            let field = self.solve_system(&self.mode_source(&port.mode, port.incoming()))?;
            for (q, at) in ports.iter().enumerate() {
                let (forward, backward) = field.mode_amplitudes(&at.mode);
                let (a, b) = match at.side {
                    Side::Left => (forward, backward),
                    Side::Right => (backward, forward),
                };
                incoming[(q, p)] = a * roots[q];
                outgoing[(q, p)] = b * roots[q];
            }
        }
        // S A = B, so Aᵀ Sᵀ = Bᵀ
        let st = incoming
            .transpose()
            .to_owned()
            .partial_piv_lu()
            .solve(outgoing.transpose().to_owned());
        Ok((0..n)
            .map(|q| (0..n).map(|p| st[(p, q)]).collect())
            .collect())
    }
}

impl Field2d {
    /// The forward and backward amplitudes of `mode` at its column (see the module's docs).
    ///
    /// # Panics
    ///
    /// If the mode's column isn't on this field's grid.
    pub fn mode_amplitudes(&self, mode: &PortMode) -> (c64, c64) {
        let c = mode.column;
        let column = |i: usize| (0..self.grid.ny).map(move |j| self.at(i, j));
        let (p0, p1) = (mode.share(column(c)), mode.share(column(c + 1)));
        let step = (c64::new(0.0, 1.0) * mode.beta * self.grid.dx).exp();
        let forward = (p1 - p0 / step) / (step - 1.0 / step);
        (forward, p0 - forward)
    }

    /// The power crossing the face between columns `i` and `i + 1` towards +x, per unit length
    /// along z: the column's sum of ½ Re(E × H̃*)_x dy, η₀ = 1 (in |field|² µm).
    ///
    /// # Panics
    ///
    /// If column `i + 1` isn't on the grid.
    pub fn flux_x(&self, i: usize) -> f64 {
        let g = self.grid;
        assert!(
            i + 1 < g.nx,
            "flux_x needs columns {i} and {} on the grid",
            i + 1
        );
        (0..g.ny)
            .map(|j| {
                let eps = match self.polarization {
                    Polarization::Ez => c64::new(1.0, 0.0),
                    Polarization::Hz => self.eps_y[j * (g.nx + 1) + i + 1],
                };
                flux(
                    self.polarization,
                    self.k0,
                    g.dx,
                    self.at(i, j),
                    self.at(i + 1, j),
                    eps,
                ) * g.dy
            })
            .sum()
    }
}
