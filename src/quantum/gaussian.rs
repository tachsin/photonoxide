//! Gaussian states of light: squeezed, displaced and thermal modes through any circuit, their
//! photon counts by hafnians and their clicks by torontonians.
//!
//! The method is in [`Gaussian`]'s documentation.

use std::f64::consts::FRAC_PI_2;

use num_complex::Complex64 as c64;

use super::hafnian::{hafnian, loop_hafnian, repeated, torontonian};
use super::matrix::{determinant, hermitian_eigen, inverse};
use super::{Matrix, Transfer};
use crate::{Error, Result};

/// The most photons a pattern may hold: a hafnian of 2 × 30 rows is 2^30 terms.
const MOST_PHOTONS: usize = 30;

/// Gaussian states of light: squeezed, displaced and thermal modes, through any circuit, with
/// their photon counts by hafnians and their clicks by torontonians.
///
/// # Conventions
///
/// - **Quadratures** x̂ = â + â†, p̂ = −i(â − â†) (ħ = 2): the vacuum's variances are 1, and a
///   variance below 1 is squeezed. A state of n modes is its means d = ⟨(x₁ … x_n, p₁ … p_n)⟩
///   and its covariance V_ij = ½⟨{Δr_i, Δr_j}⟩, the x's first (C. Weedbrook et al., Rev. Mod.
///   Phys. 84, 621 (2012), doi:10.1103/RevModPhys.84.621, Sec. II, with their ħ = 2).
/// - **A Gaussian unitary** b̂ = U â + W â† acts on the quadratures as the real symplectic
///   S = [[Re(U + W), −Im(U − W)], [Im(U + W), Re(U − W)]]: V → S V Sᵀ, d → S d.
/// - **The squeezer** S(ξ) = exp(½(ξ* â² − ξ â†²)), ξ = r e^{iφ}: â → â cosh r − â† e^{iφ} sinh r;
///   at φ = 0 it multiplies x's variance by e^{−2r} and p's by e^{2r}. **The two-mode squeezer**
///   exp(ξ* â b̂ − ξ â† b̂†) takes â → â cosh r − b̂† e^{iφ} sinh r and b̂ likewise.
/// - **A passive circuit** b̂ = T â + L ĉ with the vacuum ĉ through its loss (L L† = I − T T†,
///   [`super::Transfer`]): V → R V Rᵀ + R_L R_Lᵀ, R and R_L the real forms of T and L; R_L R_Lᵀ =
///   I − R Rᵀ.
/// - **Photon counts** (C. S. Hamilton et al., Phys. Rev. Lett. 119, 170501 (2017),
///   doi:10.1103/PhysRevLett.119.170501, Eqs. (2)–(5); with means, the loop hafnian of A.
///   Björklund, B. Gupt, N. Quesada, doi:10.1145/3325111): in the basis (â, â†), the complex
///   covariance σ = ¼ W V W† with W = [[I, iI], [I, −iI]], σ_Q = σ + ½I, A = X(I − σ_Q⁻¹) with
///   X = [[0, I], [I, 0]], β = (α, α*) the means (α = (d_x + i d_p)/2) and γ = β†σ_Q⁻¹:
///   P(n) = exp(−½ β†σ_Q⁻¹β) lhaf(Ã_n) / (√det σ_Q ∏ n_i!), Ã_n the rows and columns of A for
///   mode i and its conjugate each repeated n_i times, its diagonal γ's (the hafnian of A_n
///   without means).
/// - **Threshold clicks** (N. Quesada, J. M. Arrazola, N. Killoran, Phys. Rev. A 98, 062322
///   (2018), doi:10.1103/PhysRevA.98.062322): the modes that click S, the rest dark,
///   P(S) = Tor(O_S) / √det σ_Q, O = I − σ_Q⁻¹, for a state without means; with means, the
///   alternating sum over S's subsets Y of the probability that the dark modes and Y hold no
///   photon, each a reduced state's P(0), exp(−½ β†σ_Q⁻¹β)/√det σ_Q (inclusion and exclusion).
///
/// A Gaussian state of n modes: its quadratures' means and covariance (vacuum variance 1).
#[derive(Clone, Debug, PartialEq)]
pub struct Gaussian {
    modes: usize,
    /// V, 2n × 2n, row by row, (x₁ … x_n, p₁ … p_n).
    covariance: Vec<f64>,
    /// d, 2n.
    means: Vec<f64>,
}

fn invalid(what: impl Into<String>) -> Error {
    Error::invalid("gaussian state", what)
}

impl Gaussian {
    /// The vacuum on `modes` modes.
    pub fn vacuum(modes: usize) -> Gaussian {
        let n = 2 * modes;
        Gaussian {
            modes,
            covariance: (0..n * n)
                .map(|k| if k / n == k % n { 1.0 } else { 0.0 })
                .collect(),
            means: vec![0.0; n],
        }
    }

    /// A state of `modes` modes from its covariance (2n × 2n, row by row, x's first) and means.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for the wrong lengths, values that aren't finite, a covariance
    /// that isn't symmetric, or one that breaks the uncertainty principle V + iΩ ≥ 0 (beyond
    /// 1e-9).
    pub fn new(modes: usize, covariance: Vec<f64>, means: Vec<f64>) -> Result<Gaussian> {
        let n = 2 * modes;
        if covariance.len() != n * n || means.len() != n {
            return Err(invalid(format!(
                "{modes} modes take a {n} × {n} covariance and {n} means, got {} and {}",
                covariance.len(),
                means.len()
            )));
        }
        if covariance.iter().chain(&means).any(|v| !v.is_finite()) {
            return Err(invalid("every value must be finite"));
        }
        for i in 0..n {
            for j in 0..i {
                let (a, b) = (covariance[i * n + j], covariance[j * n + i]);
                if (a - b).abs() > 1e-12 * (a.abs() + b.abs()).max(1.0) {
                    return Err(invalid("the covariance must be symmetric"));
                }
            }
        }
        // V + iΩ ≥ 0, Ω = [[0, I], [−I, 0]]
        let h = Matrix::from_fn(n, n, |i, j| {
            let omega = if i < modes && j == i + modes {
                1.0
            } else if i >= modes && j + modes == i {
                -1.0
            } else {
                0.0
            };
            c64::new(covariance[i * n + j], omega)
        });
        let (eigenvalues, _) = hermitian_eigen(&h);
        if let Some(&low) = eigenvalues.iter().min_by(|a, b| a.total_cmp(b))
            && low < -1e-9
        {
            return Err(invalid(format!(
                "the covariance breaks the uncertainty principle: V + iΩ has the eigenvalue {low:e}"
            )));
        }
        Ok(Gaussian {
            modes,
            covariance,
            means,
        })
    }

    /// The number of modes.
    pub fn modes(&self) -> usize {
        self.modes
    }

    /// The covariance V, 2n × 2n, row by row, (x₁ … x_n, p₁ … p_n).
    pub fn covariance(&self) -> &[f64] {
        &self.covariance
    }

    /// The means d, (x₁ … x_n, p₁ … p_n).
    pub fn means(&self) -> &[f64] {
        &self.means
    }

    fn v(&self, i: usize, j: usize) -> f64 {
        self.covariance[i * 2 * self.modes + j]
    }

    fn check_mode(&self, mode: usize) -> Result<()> {
        if mode >= self.modes {
            return Err(invalid(format!(
                "mode {mode} is out of range: the state has {}",
                self.modes
            )));
        }
        Ok(())
    }

    /// Applies b̂ = U â + W â† to all the modes ([`Gaussian`]'s conventions).
    fn bogoliubov(&self, u: &Matrix, w: &Matrix) -> Gaussian {
        let n = self.modes;
        let s = Matrix::from_fn(2 * n, 2 * n, |i, j| {
            let (bi, bj) = (i / n, j / n);
            let (r, c) = (i % n, j % n);
            let plus = u[(r, c)] + w[(r, c)];
            let minus = u[(r, c)] - w[(r, c)];
            c64::new(
                match (bi, bj) {
                    (0, 0) => plus.re,
                    (0, 1) => -minus.im,
                    (1, 0) => plus.im,
                    _ => minus.re,
                },
                0.0,
            )
        });
        self.symplectic(&s, None)
    }

    /// V → S V Sᵀ + noise, d → S d, S real.
    fn symplectic(&self, s: &Matrix, noise: Option<&Matrix>) -> Gaussian {
        let n = 2 * self.modes;
        let mut covariance = vec![0.0; n * n];
        // S V
        let sv: Vec<f64> = (0..n * n)
            .map(|k| {
                let (i, j) = (k / n, k % n);
                (0..n).map(|l| s[(i, l)].re * self.v(l, j)).sum()
            })
            .collect();
        for i in 0..n {
            for j in 0..n {
                let mut x: f64 = (0..n).map(|l| sv[i * n + l] * s[(j, l)].re).sum();
                if let Some(noise) = noise {
                    x += noise[(i, j)].re;
                }
                covariance[i * n + j] = x;
            }
        }
        // symmetric to the last bit
        for i in 0..n {
            for j in 0..i {
                let mean = 0.5 * (covariance[i * n + j] + covariance[j * n + i]);
                covariance[i * n + j] = mean;
                covariance[j * n + i] = mean;
            }
        }
        let means = (0..n)
            .map(|i| (0..n).map(|l| s[(i, l)].re * self.means[l]).sum())
            .collect();
        Gaussian {
            modes: self.modes,
            covariance,
            means,
        }
    }

    /// The state with `mode` displaced by α: d_x += 2 Re α, d_p += 2 Im α.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a mode out of range or an α that isn't finite.
    pub fn displaced(&self, mode: usize, alpha: c64) -> Result<Gaussian> {
        self.check_mode(mode)?;
        if !(alpha.re.is_finite() && alpha.im.is_finite()) {
            return Err(invalid("a displacement must be finite"));
        }
        let mut g = self.clone();
        g.means[mode] += 2.0 * alpha.re;
        g.means[mode + self.modes] += 2.0 * alpha.im;
        Ok(g)
    }

    /// The state with `mode` squeezed by ξ = r e^{iφ} ([`Gaussian`]'s conventions).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a mode out of range or an r or φ that isn't finite.
    pub fn squeezed(&self, mode: usize, r: f64, phi: f64) -> Result<Gaussian> {
        self.check_mode(mode)?;
        if !(r.is_finite() && phi.is_finite()) {
            return Err(invalid("a squeezing must be finite"));
        }
        let n = self.modes;
        let u = Matrix::from_fn(n, n, |i, j| {
            let x = if i != j {
                0.0
            } else if i == mode {
                r.cosh()
            } else {
                1.0
            };
            c64::new(x, 0.0)
        });
        let w = Matrix::from_fn(n, n, |i, j| {
            if i == mode && j == mode {
                -c64::from_polar(r.sinh(), phi)
            } else {
                c64::new(0.0, 0.0)
            }
        });
        Ok(self.bogoliubov(&u, &w))
    }

    /// The state with modes `a` and `b` squeezed together by ξ = r e^{iφ`}: from the vacuum,
    /// the two-mode squeezed vacuum, each half thermal with sinh² r photons.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for modes out of range or equal, or an r or φ that isn't finite.
    pub fn two_mode_squeezed(&self, a: usize, b: usize, r: f64, phi: f64) -> Result<Gaussian> {
        self.check_mode(a)?;
        self.check_mode(b)?;
        if a == b || !(r.is_finite() && phi.is_finite()) {
            return Err(invalid(
                "two-mode squeezing takes two different modes and a finite squeezing",
            ));
        }
        let n = self.modes;
        let u = Matrix::from_fn(n, n, |i, j| {
            let x = if i != j {
                0.0
            } else if i == a || i == b {
                r.cosh()
            } else {
                1.0
            };
            c64::new(x, 0.0)
        });
        let w = Matrix::from_fn(n, n, |i, j| {
            if (i == a && j == b) || (i == b && j == a) {
                -c64::from_polar(r.sinh(), phi)
            } else {
                c64::new(0.0, 0.0)
            }
        });
        Ok(self.bogoliubov(&u, &w))
    }

    /// The state with `mode` replaced by a thermal state of `mean` photons, variance 2n̄ + 1.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a mode out of range or a mean that isn't finite and ≥ 0.
    pub fn with_thermal(&self, mode: usize, mean: f64) -> Result<Gaussian> {
        self.check_mode(mode)?;
        if !(mean.is_finite() && mean >= 0.0) {
            return Err(invalid("a thermal state's mean must be finite and ≥ 0"));
        }
        let n = 2 * self.modes;
        let mut g = self.clone();
        for q in [mode, mode + self.modes] {
            for k in 0..n {
                g.covariance[q * n + k] = 0.0;
                g.covariance[k * n + q] = 0.0;
            }
            g.covariance[q * n + q] = 2.0 * mean + 1.0;
            g.means[q] = 0.0;
        }
        Ok(g)
    }

    /// The state after a passive linear map b̂ = T â on all its modes, T n × n with T T† ≤ I,
    /// the vacuum entering through its loss ([`Gaussian`]'s conventions).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless T is n × n and passive (the eigenvalues of T†T at most
    /// 1 + 1e-9).
    pub fn transformed(&self, t: &Matrix) -> Result<Gaussian> {
        let n = self.modes;
        if t.rows() != n || t.cols() != n {
            return Err(invalid(format!(
                "a map of {} × {} on {n} modes",
                t.rows(),
                t.cols()
            )));
        }
        let (eigenvalues, _) = hermitian_eigen(&t.adjoint().product(t)?);
        if eigenvalues.iter().any(|&e| e > 1.0 + 1e-9) {
            return Err(invalid(
                "the map has gain: a passive circuit's T†T is at most I",
            ));
        }
        let real = |m: &Matrix| {
            Matrix::from_fn(2 * n, 2 * n, |i, j| {
                let v = m[(i % n, j % n)];
                c64::new(
                    match (i / n, j / n) {
                        (0, 0) | (1, 1) => v.re,
                        (0, 1) => -v.im,
                        _ => v.im,
                    },
                    0.0,
                )
            })
        };
        let r = real(t);
        let mut noise = Matrix::identity(2 * n);
        let rrt = r.product(&r.adjoint())?;
        for i in 0..2 * n {
            for j in 0..2 * n {
                noise[(i, j)] -= rrt[(i, j)];
            }
        }
        Ok(self.symplectic(&r, Some(&noise)))
    }

    /// The state on a circuit's ports after its transfer ([`Transfer`]): one mode per port, in
    /// the transfer's order.
    ///
    /// # Errors
    ///
    /// As [`Gaussian::transformed`]; the state must have one mode per port.
    pub fn through(&self, transfer: &Transfer) -> Result<Gaussian> {
        self.transformed(transfer.matrix())
    }

    /// The state with `mode` sent through a loss of transmission `eta` (0 to 1): x's variance
    /// v becomes η v + 1 − η.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a mode out of range or a transmission outside 0 to 1.
    pub fn lossy(&self, mode: usize, eta: f64) -> Result<Gaussian> {
        self.check_mode(mode)?;
        if !(0.0..=1.0).contains(&eta) {
            return Err(invalid(format!(
                "a transmission must be from 0 to 1, got {eta}"
            )));
        }
        let n = self.modes;
        let t = Matrix::from_fn(n, n, |i, j| {
            let x = if i != j {
                0.0
            } else if i == mode {
                eta.sqrt()
            } else {
                1.0
            };
            c64::new(x, 0.0)
        });
        self.transformed(&t)
    }

    /// The state of the modes listed, in that order: the rest traced out.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a mode out of range or listed twice.
    pub fn reduced(&self, modes: &[usize]) -> Result<Gaussian> {
        for (k, &m) in modes.iter().enumerate() {
            self.check_mode(m)?;
            if modes[..k].contains(&m) {
                return Err(invalid(format!("mode {m} is listed twice")));
            }
        }
        let k = modes.len();
        let index = |i: usize| {
            if i < k {
                modes[i]
            } else {
                modes[i - k] + self.modes
            }
        };
        Ok(Gaussian {
            modes: k,
            covariance: (0..4 * k * k)
                .map(|q| self.v(index(q / (2 * k)), index(q % (2 * k))))
                .collect(),
            means: (0..2 * k).map(|i| self.means[index(i)]).collect(),
        })
    }

    /// The variance of the quadrature x cos θ + p sin θ of `mode`: what a homodyne detector
    /// with its local oscillator at phase θ measures, the vacuum's 1.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a mode out of range.
    pub fn quadrature_variance(&self, mode: usize, theta: f64) -> Result<f64> {
        self.check_mode(mode)?;
        let (x, p) = (mode, mode + self.modes);
        let (c, s) = (theta.cos(), theta.sin());
        Ok(c * c * self.v(x, x) + s * s * self.v(p, p) + 2.0 * c * s * self.v(x, p))
    }

    /// The smallest and largest quadrature variances of `mode` over the phase, the eigenvalues
    /// of its 2 × 2 covariance: squeezed below 1, anti-squeezed above.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a mode out of range.
    pub fn quadrature_extremes(&self, mode: usize) -> Result<(f64, f64)> {
        self.check_mode(mode)?;
        let (x, p) = (mode, mode + self.modes);
        let (a, b, c) = (self.v(x, x), self.v(p, p), self.v(x, p));
        let mean = 0.5 * (a + b);
        let half = (0.25 * (a - b) * (a - b) + c * c).sqrt();
        Ok((mean - half, mean + half))
    }

    /// The mean number of photons in `mode`: (V_xx + V_pp + d_x² + d_p² − 2)/4.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a mode out of range.
    pub fn mean_photons(&self, mode: usize) -> Result<f64> {
        self.check_mode(mode)?;
        let (x, p) = (mode, mode + self.modes);
        Ok(
            (self.v(x, x) + self.v(p, p) + self.means[x].powi(2) + self.means[p].powi(2) - 2.0)
                / 4.0,
        )
    }

    /// The covariance of the photon numbers of modes `a` and `b`, ⟨n_a n_b⟩ − ⟨n_a⟩⟨n_b⟩: for
    /// a ≠ b, Tr(V_ab V_abᵀ)/8 + d_aᵀ V_ab d_b / 4 (Wick's theorem, ħ = 2), and for a = b the
    /// variance (Tr(V_aa²) − 2)/8 + d_aᵀ V_aa d_a / 4.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a mode out of range.
    pub fn photon_covariance(&self, a: usize, b: usize) -> Result<f64> {
        self.check_mode(a)?;
        self.check_mode(b)?;
        let n = self.modes;
        let ia = [a, a + n];
        let ib = [b, b + n];
        let block = |i: usize, j: usize| self.v(ia[i], ib[j]);
        let trace: f64 = (0..2)
            .flat_map(|i| (0..2).map(move |j| (i, j)))
            .map(|(i, j)| block(i, j) * block(i, j))
            .sum();
        let quadratic: f64 = (0..2)
            .flat_map(|i| (0..2).map(move |j| (i, j)))
            .map(|(i, j)| self.means[ia[i]] * block(i, j) * self.means[ib[j]])
            .sum();
        let vacuum = if a == b { 2.0 } else { 0.0 };
        Ok((trace - vacuum) / 8.0 + quadratic / 4.0)
    }

    /// The purity Tr ρ² = 1/√det V.
    pub fn purity(&self) -> f64 {
        let n = 2 * self.modes;
        let v = Matrix::from_fn(n, n, |i, j| c64::new(self.v(i, j), 0.0));
        1.0 / determinant(&v).re.sqrt()
    }

    /// σ_Q and its inverse, in the basis (â, â†).
    fn q_matrices(&self) -> Result<(Matrix, Matrix)> {
        let n = self.modes;
        let i = c64::new(0.0, 1.0);
        let w = Matrix::from_fn(2 * n, 2 * n, |r, c| {
            if r % n != c % n {
                c64::new(0.0, 0.0)
            } else if c < n {
                c64::new(1.0, 0.0)
            } else if r < n {
                i
            } else {
                -i
            }
        });
        let v = Matrix::from_fn(2 * n, 2 * n, |r, c| c64::new(self.v(r, c), 0.0));
        let sigma = w.product(&v)?.product(&w.adjoint())?;
        let q = Matrix::from_fn(2 * n, 2 * n, |r, c| {
            sigma[(r, c)] * 0.25 + if r == c { 0.5 } else { 0.0 }
        });
        let inv = inverse(&q).ok_or_else(|| invalid("σ_Q is singular"))?;
        Ok((q, inv))
    }

    /// The probability of `pattern` photons in the modes, by a hafnian (or a loop hafnian with
    /// means) ([`Gaussian`]'s conventions).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a pattern of the wrong length or more than 30 photons.
    pub fn probability(&self, pattern: &[usize]) -> Result<f64> {
        let n = self.modes;
        if pattern.len() != n {
            return Err(invalid(format!(
                "a pattern of {} modes for a state of {n}",
                pattern.len()
            )));
        }
        let total: usize = pattern.iter().sum();
        if total > MOST_PHOTONS {
            return Err(invalid(format!(
                "a pattern takes at most {MOST_PHOTONS} photons, got {total}"
            )));
        }
        let (q, qinv) = self.q_matrices()?;
        // A = X (I − σ_Q⁻¹): row r of A is row r ± n of I − σ_Q⁻¹; symmetric for a physical
        // state, made so to the last bit
        let x = |r: usize, c: usize| {
            let rr = (r + n) % (2 * n);
            let one = if rr == c { 1.0 } else { 0.0 };
            c64::new(one, 0.0) - qinv[(rr, c)]
        };
        let a = Matrix::from_fn(2 * n, 2 * n, |r, c| 0.5 * (x(r, c) + x(c, r)));
        let index: Vec<usize> = (0..n)
            .flat_map(|m| std::iter::repeat_n(m, pattern[m]))
            .chain((0..n).flat_map(|m| std::iter::repeat_n(m + n, pattern[m])))
            .collect();
        let factorials: f64 = pattern
            .iter()
            .map(|&k| (1..=k).map(|j| j as f64).product::<f64>())
            .product();
        let norm = determinant(&q).re.sqrt();
        let has_means = self.means.iter().any(|&d| d != 0.0);
        // photons crowded into few modes: the moments of a sum over ∏(r + 1) terms; spread
        // out: Björklund et al.'s sum over 2^total subsets
        let distinct: Vec<usize> = (0..n)
            .filter(|&m| pattern[m] > 0)
            .chain((0..n).filter(|&m| pattern[m] > 0).map(|m| m + n))
            .collect();
        let repeats: Vec<usize> = distinct.iter().map(|&i| pattern[i % n]).collect();
        let kan: f64 = repeats.iter().map(|&r| (r + 1) as f64).product();
        let bjorklund = 2f64.powi(i32::try_from(total).unwrap_or(i32::MAX)) * (2 * total) as f64;
        let crowded = kan <= bjorklund;
        if !has_means {
            let value = if crowded {
                repeated(&a.select(&distinct, &distinct), None, &repeats)
            } else {
                hafnian(&a.select(&index, &index))?
            };
            return Ok((value.re / (norm * factorials)).max(0.0));
        }
        let alpha: Vec<c64> = (0..n)
            .map(|m| c64::new(self.means[m], self.means[m + n]) * 0.5)
            .collect();
        let beta: Vec<c64> = alpha
            .iter()
            .copied()
            .chain(alpha.iter().map(c64::conj))
            .collect();
        // γ = β† σ_Q⁻¹, and β† σ_Q⁻¹ β
        let gamma: Vec<c64> = (0..2 * n)
            .map(|c| (0..2 * n).map(|r| beta[r].conj() * qinv[(r, c)]).sum())
            .collect();
        let quadratic: c64 = (0..2 * n).map(|c| gamma[c] * beta[c]).sum();
        let prefactor = (-0.5 * quadratic.re).exp() / (norm * factorials);
        let value = if crowded {
            let loops: Vec<c64> = distinct.iter().map(|&i| gamma[i]).collect();
            repeated(&a.select(&distinct, &distinct), Some(&loops), &repeats)
        } else {
            let mut sub = a.select(&index, &index);
            for (k, &i) in index.iter().enumerate() {
                sub[(k, k)] = gamma[i];
            }
            loop_hafnian(&sub)?
        };
        Ok((value.re * prefactor).max(0.0))
    }

    /// The probability that exactly the modes in `clicks` click on threshold detectors and the
    /// rest stay dark ([`Gaussian`]'s conventions): by a torontonian for a state without means,
    /// and for one with means by its loop form, the sum over the clicking modes' subsets Y of
    /// (−1)^|Y| times the probability that the dark modes and Y all hold no photon, each the
    /// vacuum probability of the state reduced to them, exp(−½ β† σ_Q⁻¹ β)/√det σ_Q.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a pattern of the wrong length, or more than 30 clicking modes.
    pub fn click_probability(&self, clicks: &[bool]) -> Result<f64> {
        let n = self.modes;
        if clicks.len() != n {
            return Err(invalid(format!(
                "a click pattern of {} modes for a state of {n}",
                clicks.len()
            )));
        }
        if self.means.iter().any(|&d| d != 0.0) {
            return self.click_probability_with_means(clicks);
        }
        let (q, qinv) = self.q_matrices()?;
        let on: Vec<usize> = (0..n).filter(|&m| clicks[m]).collect();
        let index: Vec<usize> = on.iter().copied().chain(on.iter().map(|m| m + n)).collect();
        let o = Matrix::from_fn(index.len(), index.len(), |r, c| {
            let one = if r == c { 1.0 } else { 0.0 };
            c64::new(one, 0.0) - qinv[(index[r], index[c])]
        });
        let norm = determinant(&q).re.sqrt();
        Ok((torontonian(&o)?.re / norm).clamp(0.0, 1.0))
    }

    /// [`Gaussian::click_probability`] of a state with means: inclusion and exclusion over the
    /// clicking modes, each term a reduced state's vacuum probability (a loop hafnian of no
    /// rows), the 2^k terms added in order, Kahan-summed.
    fn click_probability_with_means(&self, clicks: &[bool]) -> Result<f64> {
        let n = self.modes;
        let on: Vec<usize> = (0..n).filter(|&m| clicks[m]).collect();
        let dark: Vec<usize> = (0..n).filter(|&m| !clicks[m]).collect();
        if on.len() > MOST_PHOTONS {
            return Err(invalid(format!(
                "a click pattern takes at most {MOST_PHOTONS} clicking modes, got {}",
                on.len()
            )));
        }
        let mut sum = super::permanent::Kahan::default();
        for y in 0u64..1 << on.len() {
            let mut modes = dark.clone();
            modes.extend((0..on.len()).filter(|&i| y >> i & 1 == 1).map(|i| on[i]));
            // no modes left: the empty product, 1
            let vacuum = if modes.is_empty() {
                c64::new(1.0, 0.0)
            } else {
                let empty = vec![0; modes.len()];
                c64::new(self.reduced(&modes)?.probability(&empty)?, 0.0)
            };
            sum.add(if y.count_ones() % 2 == 1 {
                -vacuum
            } else {
                vacuum
            });
        }
        Ok(sum.sum.re.clamp(0.0, 1.0))
    }

    /// The Wigner function of `mode` (the others traced out) at the quadratures `x` and `p`:
    /// W(r) = exp(−½ (r − d)ᵀ V⁻¹ (r − d)) / (2π √det V), V and d the mode's 2 × 2 covariance
    /// and means (ħ = 2: the vacuum's W(0, 0) is 1/(2π), and ∫∫ W dx dp = 1). A Gaussian
    /// state's Wigner function is this Gaussian, never negative (Weedbrook et al., Sec. II).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a mode out of range.
    pub fn wigner(&self, mode: usize, x: f64, p: f64) -> Result<f64> {
        self.check_mode(mode)?;
        let (ix, ip) = (mode, mode + self.modes);
        let (a, b, c) = (self.v(ix, ix), self.v(ip, ip), self.v(ix, ip));
        let det = a * b - c * c;
        let (dx, dp) = (x - self.means[ix], p - self.means[ip]);
        // (r − d)ᵀ V⁻¹ (r − d) with V⁻¹ = [[b, −c], [−c, a]] / det
        let q = (b * dx * dx - 2.0 * c * dx * dp + a * dp * dp) / det;
        Ok((-0.5 * q).exp() / (std::f64::consts::TAU * det.sqrt()))
    }

    /// The squeezing of `mode` in dB, 10 log₁₀ of its smallest quadrature variance: negative
    /// when squeezed.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a mode out of range.
    pub fn squeezing_db(&self, mode: usize) -> Result<f64> {
        Ok(10.0 * self.quadrature_extremes(mode)?.0.log10())
    }
}

/// The squeezing r of a variance e^{−2r} in dB: 20 r log₁₀ e.
pub fn db_of_squeezing(r: f64) -> f64 {
    20.0 * r * std::f64::consts::LOG10_E
}

/// The phase of the quadrature x cos θ + p sin θ that a squeezer ξ = r e^{iφ} squeezes: θ = φ/2.
pub fn squeezed_quadrature(phi: f64) -> f64 {
    0.5 * phi
}

/// The phase of the quadrature it anti-squeezes, θ = φ/2 + π/2.
pub fn anti_squeezed_quadrature(phi: f64) -> f64 {
    0.5 * phi + FRAC_PI_2
}
