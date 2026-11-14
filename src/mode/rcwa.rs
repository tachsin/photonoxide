//! A plane wave's diffraction by a stack of one-dimensional lamellar gratings, by rigorous
//! coupled-wave analysis (RCWA): the Fourier modal method.
//!
//! Layers lie between a semi-infinite cover and substrate; each is uniform, or lamellar: stripes
//! of complex index n + iκ across one period Λ along x, invariant along y and through the layer's
//! thickness along z. A plane wave of vacuum wavelength λ comes from the cover in the xz plane at
//! angle θ, TE (E along y, along the stripes) or TM (H along y), with e^(−iωt) (κ ≥ 0 for loss),
//! photonoxide's convention. docs/methods/rcwa.md has the derivation.
//!
//! - **The modes of a layer.** The fields are Floquet series, Σ_m S_m(z) e^(ik_m x), with
//!   k_m = k₀ n_c sin θ + 2πm/Λ, truncated to |m| ≤ M (2M + 1 harmonics); in units of k₀,
//!   K_x = diag(k_m/k₀) and z′ = k₀z. E = ⟦ε⟧ and A = ⟦1/ε⟧ are the Toeplitz matrices of the
//!   Fourier coefficients of ε(x) and 1/ε(x). TE (Moharam and Gaylord 1981): S″ = (K_x² − E) S.
//!   TM, by Li's factorization rules (Li 1996, Eq. 4; the form Lalanne and Morris, and Granet
//!   and Guizal, found in 1996): H″ = A⁻¹(K_x E⁻¹ K_x − I) H, the inverse rule wherever ε
//!   multiplies a field whose product with it is continuous across the stripes' walls. The
//!   eigenvectors W and eigenvalues q² of these matrices are the layer's modes, e^(∓qz′); q takes
//!   the root that decays downward (Re q > 0), or for a propagating mode the one that travels
//!   downward. The tangential fields are W(c⁺ + c⁻) and V(−c⁺ + c⁻): V = WQ for TE (∂S/∂z′, the
//!   tangential H), V = AWQ for TM (A ∂H/∂z′, the tangential E).
//! - **The stack.** Each interface's scattering matrix follows from the two layers' (W, V); each
//!   layer's is its modes' e^(−qd) both ways; the stack's is their Redheffer star product, with
//!   only decaying exponentials anywhere, so deep and lossy layers stay stable (the S-matrix
//!   algorithm: L. Li, J. Opt. Soc. Am. A 13, 1024 (1996), in place of Moharam et al. 1995's
//!   enhanced transmittance matrix, which it matches).
//! - **Efficiencies.** The cover's reflected and the substrate's transmitted amplitudes of each
//!   order give its share of the incident power: |r_m|² Re(k_z,m)/k_z,0 for TE, with k_z/ε for
//!   TM; what is neither reflected nor transmitted is absorbed.
//!
//! A uniform layer is exact at any M (its modes are the plane waves, W = I), so a stack of them
//! is [`super::multilayer::Multilayer`]'s to round-off. A lamellar layer converges as M grows.

use std::f64::consts::PI;

use faer::diag::Diag;
use faer::dyn_stack::{MemBuffer, MemStack};
use faer::linalg::evd::{self, ComputeEigenvectors};
use faer::linalg::solvers::Solve;
use faer::{Mat, Par};
use num_complex::Complex64 as c64;

use super::Polarization;
use crate::portable;
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

mod tangent;
pub use tangent::{DiffractionTangent, GratingParameter};

/// The most harmonics either side of the zeroth that [`Grating::diffract`] keeps.
pub const MAX_ORDERS: usize = 400;

/// A layer of a [`Grating`]: stripes of complex index across one period, and its thickness.
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    /// (index, width in µm) from x = 0; one stripe for a uniform layer.
    stripes: Vec<(c64, f64)>,
    /// µm.
    thickness: f64,
}

impl Layer {
    /// A uniform layer of `index`, `thickness` thick.
    pub fn uniform(index: c64, thickness: Length) -> Layer {
        Layer {
            stripes: vec![(index, f64::NAN)],
            thickness: thickness.to_um(),
        }
    }

    /// A lamellar layer: `stripes` of (index, width) from x = 0 across one period, in order (their
    /// widths sum to the grating's period), `thickness` thick.
    pub fn lamellar(stripes: &[(c64, Length)], thickness: Length) -> Layer {
        Layer {
            stripes: stripes.iter().map(|&(n, w)| (n, w.to_um())).collect(),
            thickness: thickness.to_um(),
        }
    }

    /// The layer's thickness.
    pub fn thickness(&self) -> Length {
        Length::um(self.thickness)
    }

    /// Whether every stripe has the same index: the layer is uniform.
    fn is_uniform(&self) -> bool {
        self.stripes.iter().all(|s| s.0 == self.stripes[0].0)
    }
}

/// A stack of [`Layer`]s with one period between a cover and a substrate.
#[derive(Clone, Debug, PartialEq)]
pub struct Grating {
    period: f64,
    cover: c64,
    layers: Vec<Layer>,
    substrate: c64,
}

/// How a TM layer's matrix is factorized: Li's rules, or Laurent's rule throughout (the 1981
/// formulation, E in place of A⁻¹), kept to show what the rules buy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Factorization {
    Li,
    Laurent,
}

/// A plane wave's diffraction by a [`Grating`] ([`Grating::diffract`]).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Diffraction {
    /// The orders kept, −M … M.
    pub orders: Vec<i32>,
    /// Each order's reflected amplitude: of the tangential E (TE) or H (TM), against the
    /// incident wave's, at the cover's side of the stack.
    pub r: Vec<c64>,
    /// Each order's transmitted amplitude, of the same component, at the substrate's side.
    pub t: Vec<c64>,
    /// Each order's share of the incident power reflected (zero for an evanescent order).
    pub reflected: Vec<f64>,
    /// Each order's share transmitted into the substrate.
    pub transmitted: Vec<f64>,
    /// The share reflected, all orders.
    pub reflectance: f64,
    /// The share transmitted, all orders.
    pub transmittance: f64,
    /// The share absorbed in the layers, 1 − reflectance − transmittance.
    pub absorption: f64,
}

impl Diffraction {
    /// The reflected share of order `m`, zero if it isn't kept.
    pub fn reflected_order(&self, m: i32) -> f64 {
        self.index(m).map_or(0.0, |i| self.reflected[i])
    }

    /// The transmitted share of order `m`, zero if it isn't kept.
    pub fn transmitted_order(&self, m: i32) -> f64 {
        self.index(m).map_or(0.0, |i| self.transmitted[i])
    }

    fn index(&self, m: i32) -> Option<usize> {
        self.orders.iter().position(|&o| o == m)
    }
}

/// A layer's modes at one wavelength: the tangential fields' matrices W and V and the modes'
/// q (their e^(−qz′)).
struct Modes {
    w: Mat<c64>,
    v: Mat<c64>,
    q: Vec<c64>,
}

/// A scattering matrix: s11 reflects from above, s21 transmits down, s12 up, s22 reflects from
/// below.
struct Scattering {
    s11: Mat<c64>,
    s12: Mat<c64>,
    s21: Mat<c64>,
    s22: Mat<c64>,
}

impl Grating {
    /// `layers` (from the cover's side) of one `period` between `cover` and `substrate`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a period or thickness that isn't positive and finite, an
    /// index that isn't finite or has a real part that isn't positive, a stripe whose width
    /// isn't positive, or a lamellar layer whose widths don't sum to the period (to 1e-9 of it).
    pub fn new(period: Length, cover: c64, layers: &[Layer], substrate: c64) -> Result<Grating> {
        let p = period.to_um();
        if !(p.is_finite() && p > 0.0) {
            return Err(Error::invalid(
                "grating",
                format!("the period must be positive, got {period}"),
            ));
        }
        let ok = |n: c64| n.re.is_finite() && n.im.is_finite() && n.re > 0.0;
        for &n in [cover, substrate]
            .iter()
            .chain(layers.iter().flat_map(|l| l.stripes.iter().map(|s| &s.0)))
        {
            if !ok(n) {
                return Err(Error::invalid(
                    "grating",
                    format!("indices must be finite with a positive real part, got {n}"),
                ));
            }
        }
        let mut out = Vec::with_capacity(layers.len());
        for layer in layers {
            if !(layer.thickness.is_finite() && layer.thickness > 0.0) {
                return Err(Error::invalid(
                    "grating",
                    format!(
                        "layer thicknesses must be positive, got {} um",
                        layer.thickness
                    ),
                ));
            }
            let stripes = if layer.stripes.len() == 1 && layer.stripes[0].1.is_nan() {
                vec![(layer.stripes[0].0, p)]
            } else {
                if layer.stripes.is_empty()
                    || layer
                        .stripes
                        .iter()
                        .any(|s| !(s.1.is_finite() && s.1 > 0.0))
                {
                    return Err(Error::invalid(
                        "grating",
                        "a lamellar layer needs stripes of positive width",
                    ));
                }
                let sum: f64 = layer.stripes.iter().map(|s| s.1).sum();
                if (sum - p).abs() > 1e-9 * p {
                    return Err(Error::invalid(
                        "grating",
                        format!("a lamellar layer's widths sum to {sum} um, not the period {p} um"),
                    ));
                }
                layer.stripes.clone()
            };
            out.push(Layer {
                stripes,
                thickness: layer.thickness,
            });
        }
        Ok(Grating {
            period: p,
            cover,
            layers: out,
            substrate,
        })
    }

    /// The period.
    pub fn period(&self) -> Length {
        Length::um(self.period)
    }

    /// The layers, from the cover's side.
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// The diffraction of a plane wave of `polarization` incident from the cover at `angle`
    /// (radians from the normal, in the xz plane), with the orders −`orders` … `orders` kept
    /// (2 `orders` + 1 harmonics).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a lossy cover (an incident plane wave in it isn't defined), an
    /// angle outside [0, π/2), more orders than [`MAX_ORDERS`], an order exactly at grazing in
    /// the cover or substrate (a Wood anomaly, where its field matrix is singular), or an
    /// eigensolver that fails.
    pub fn diffract(
        &self,
        polarization: Polarization,
        wavelength: Wavelength,
        angle: f64,
        orders: usize,
    ) -> Result<Diffraction> {
        self.diffract_with(polarization, wavelength, angle, orders, Factorization::Li)
    }

    /// [`Grating::diffract`] with a choice of the TM factorization.
    pub(crate) fn diffract_with(
        &self,
        polarization: Polarization,
        wavelength: Wavelength,
        angle: f64,
        orders: usize,
        rule: Factorization,
    ) -> Result<Diffraction> {
        if self.cover.im != 0.0 {
            return Err(Error::invalid(
                "diffraction",
                format!("the cover must be lossless, its index is {}", self.cover),
            ));
        }
        if !(angle.is_finite() && (0.0..PI / 2.0).contains(&angle)) {
            return Err(Error::invalid(
                "diffraction",
                format!("the angle must be in [0, pi/2) radians, got {angle}"),
            ));
        }
        if orders > MAX_ORDERS {
            return Err(Error::invalid(
                "diffraction",
                format!("at most {MAX_ORDERS} orders either side, got {orders}"),
            ));
        }
        let lambda = wavelength.to_um();
        let k0 = 2.0 * PI / lambda;
        let m = orders as i32;
        let kx: Vec<f64> = (-m..=m)
            .map(|o| self.cover.re * portable::sin(angle) + f64::from(o) * lambda / self.period)
            .collect();
        let te = polarization == Polarization::Te;
        let cover = uniform_modes(self.cover * self.cover, &kx, te)?;
        let substrate = uniform_modes(self.substrate * self.substrate, &kx, te)?;
        let mut modes = Vec::with_capacity(self.layers.len());
        for layer in &self.layers {
            modes.push(if layer.is_uniform() {
                let n = layer.stripes[0].0;
                uniform_modes(n * n, &kx, te)?
            } else {
                lamellar_modes(&layer.stripes, self.period, &kx, te, rule)?
            });
        }
        // the cover's interface, then each layer's propagation and its lower interface
        let mut above = &cover;
        let mut s: Option<Scattering> = None;
        for (layer, here) in self.layers.iter().zip(&modes) {
            let i = interface(above, here)?;
            let joined = match s {
                None => i,
                Some(s) => star(&s, &i)?,
            };
            let x: Vec<c64> = here
                .q
                .iter()
                .map(|&q| portable::complex::exp(-q * k0 * layer.thickness))
                .collect();
            s = Some(propagate(joined, &x));
            above = here;
        }
        let last = interface(above, &substrate)?;
        let s = match s {
            None => last,
            Some(s) => star(&s, &last)?,
        };
        let centre = orders;
        let size = 2 * orders + 1;
        let r: Vec<c64> = (0..size).map(|i| s.s11[(i, centre)]).collect();
        let t: Vec<c64> = (0..size).map(|i| s.s21[(i, centre)]).collect();
        // the flux of each order: Re(k_z), or Re(k_z/ε) for TM; k_z = iq
        let flux = |q: c64, eps: c64| {
            let kz = c64::new(0.0, 1.0) * q;
            if te { kz.re } else { (kz / eps).re }
        };
        let (eps_c, eps_s) = (self.cover * self.cover, self.substrate * self.substrate);
        let incident = flux(cover.q[centre], eps_c);
        let reflected: Vec<f64> = (0..size)
            .map(|i| r[i].norm_sqr() * flux(cover.q[i], eps_c) / incident)
            .collect();
        let transmitted: Vec<f64> = (0..size)
            .map(|i| t[i].norm_sqr() * flux(substrate.q[i], eps_s) / incident)
            .collect();
        let reflectance: f64 = reflected.iter().sum();
        let transmittance: f64 = transmitted.iter().sum();
        Ok(Diffraction {
            orders: (-m..=m).collect(),
            r,
            t,
            reflected,
            transmitted,
            reflectance,
            transmittance,
            absorption: 1.0 - reflectance - transmittance,
        })
    }
}

/// q for a mode of eigenvalue q² = `q2`: q = −ik_z, k_z = (−q²)^½ on the root that decays
/// downward (Im k_z > 0), or for a propagating mode (Im k_z zero to rounding) travels downward.
fn q_of(q2: c64) -> c64 {
    let kz = portable::complex::sqrt(-q2);
    let down = if kz.im.abs() > 1e-12 * portable::complex::abs(kz) {
        kz.im > 0.0
    } else {
        kz.re >= 0.0
    };
    let kz = if down { kz } else { -kz };
    c64::new(0.0, -1.0) * kz
}

/// A uniform medium's modes: its plane waves, W = I, q_m² = k_m² − ε.
fn uniform_modes(eps: c64, kx: &[f64], te: bool) -> Result<Modes> {
    let n = kx.len();
    let q: Vec<c64> = kx.iter().map(|&k| q_of(k * k - eps)).collect();
    if q.iter().any(|q| portable::complex::abs(*q) == 0.0) {
        return Err(Error::invalid(
            "diffraction",
            "an order is exactly at grazing (a Wood anomaly): move the wavelength or angle slightly",
        ));
    }
    let v = Mat::<c64>::from_fn(n, n, |i, j| {
        if i == j {
            if te { q[i] } else { q[i] / eps }
        } else {
            c64::new(0.0, 0.0)
        }
    });
    Ok(Modes {
        w: Mat::<c64>::identity(n, n),
        v,
        q,
    })
}

/// The Fourier coefficients ε_h, h = −2M … 2M, of a lamellar profile's `values` (ε or 1/ε per
/// stripe) over `period`: ε_h = (1/Λ) ∫ ε(x) e^(−i2πhx/Λ) dx.
fn coefficients(values: &[(c64, f64)], period: f64, size: usize) -> Vec<c64> {
    let top = size as i64 - 1;
    (-top..=top)
        .map(|h| {
            let mut x = 0.0;
            let mut sum = c64::new(0.0, 0.0);
            for &(e, w) in values {
                let (a, b) = (x / period, (x + w) / period);
                sum += if h == 0 {
                    e * (b - a)
                } else {
                    let phase =
                        |u: f64| portable::complex::from_polar(1.0, -2.0 * PI * h as f64 * u);
                    e * (phase(a) - phase(b)) / c64::new(0.0, 2.0 * PI * h as f64)
                };
                x += w;
            }
            sum
        })
        .collect()
}

/// The Toeplitz matrix of `c` (h = −(size−1) … size−1): (m, n) ↦ c_(m−n).
fn toeplitz(c: &[c64], size: usize) -> Mat<c64> {
    Mat::<c64>::from_fn(size, size, |i, j| c[i + size - 1 - j])
}

/// A lamellar layer's modes: TE from K_x² − E; TM from A⁻¹(K_x E⁻¹ K_x − I) with V = AWQ
/// (Li's rules, his Eq. 4), or from E(K_x E⁻¹ K_x − I) with V = E⁻¹WQ (Laurent's rule where Li's
/// inverse rule belongs: Moharam et al. 1995's form, Li's Eq. 5).
fn lamellar_modes(
    stripes: &[(c64, f64)],
    period: f64,
    kx: &[f64],
    te: bool,
    rule: Factorization,
) -> Result<Modes> {
    lamellar_parts(stripes, period, kx, te, rule).map(|(modes, _)| modes)
}

/// What a lamellar layer's modes were found from, for their derivatives: E, E⁻¹ (TM), the
/// matrix and its eigenvalues q².
struct Lamellar {
    e_inv: Option<Mat<c64>>,
    matrix: Mat<c64>,
    q2: Vec<c64>,
}

/// [`lamellar_modes`], with what they were found from.
fn lamellar_parts(
    stripes: &[(c64, f64)],
    period: f64,
    kx: &[f64],
    te: bool,
    rule: Factorization,
) -> Result<(Modes, Lamellar)> {
    let n = kx.len();
    let eps: Vec<(c64, f64)> = stripes.iter().map(|&(m, w)| (m * m, w)).collect();
    let e = toeplitz(&coefficients(&eps, period, n), n);
    let mut e_inverse = None;
    let matrix = if te {
        Mat::<c64>::from_fn(n, n, |i, j| {
            let d = if i == j { kx[i] * kx[i] } else { 0.0 };
            c64::new(d, 0.0) - e[(i, j)]
        })
    } else {
        // K_x E⁻¹ K_x − I
        let e_inv = e.partial_piv_lu().solve(Mat::<c64>::identity(n, n));
        let b = Mat::<c64>::from_fn(n, n, |i, j| {
            let d = if i == j { 1.0 } else { 0.0 };
            e_inv[(i, j)] * kx[i] * kx[j] - d
        });
        e_inverse = Some(e_inv);
        match rule {
            Factorization::Li => inverse_rule(stripes, period, n).partial_piv_lu().solve(&b),
            Factorization::Laurent => &e * &b,
        }
    };
    let (q2, w) = eigenpairs(&matrix)?;
    let q: Vec<c64> = q2.iter().copied().map(q_of).collect();
    let wq = Mat::<c64>::from_fn(n, n, |i, j| w[(i, j)] * q[j]);
    let v = if te {
        wq
    } else {
        match rule {
            Factorization::Li => &inverse_rule(stripes, period, n) * &wq,
            Factorization::Laurent => e.partial_piv_lu().solve(&wq),
        }
    };
    Ok((
        Modes { w, v, q },
        Lamellar {
            e_inv: e_inverse,
            matrix,
            q2,
        },
    ))
}

/// A = ⟦1/ε⟧, the Toeplitz matrix of 1/ε's coefficients.
fn inverse_rule(stripes: &[(c64, f64)], period: f64, n: usize) -> Mat<c64> {
    let inv: Vec<(c64, f64)> = stripes
        .iter()
        .map(|&(m, w)| (c64::new(1.0, 0.0) / (m * m), w))
        .collect();
    toeplitz(&coefficients(&inv, period, n), n)
}

/// Every eigenpair of the dense `a`, on this thread: the eigenvalues and the eigenvectors as
/// columns.
fn eigenpairs(a: &Mat<c64>) -> Result<(Vec<c64>, Mat<c64>)> {
    let n = a.nrows();
    let par = Par::Seq;
    let mut s = Diag::<c64>::zeros(n);
    let mut u = Mat::<c64>::zeros(n, n);
    let mut mem = MemBuffer::new(evd::evd_scratch::<c64>(
        n,
        ComputeEigenvectors::No,
        ComputeEigenvectors::Yes,
        par,
        Default::default(),
    ));
    evd::evd_cplx::<f64>(
        a.as_ref(),
        s.as_mut(),
        None,
        Some(u.as_mut()),
        par,
        MemStack::new(&mut mem),
        Default::default(),
    )
    .map_err(|e| Error::invalid("diffraction", format!("the eigensolver failed: {e:?}")))?;
    Ok(((0..n).map(|k| s[k]).collect(), u))
}

/// The scattering matrix of the interface from `upper` to `lower`: with a = W₁⁻¹W₂ and
/// b = V₁⁻¹V₂, the upper layer's amplitudes (c⁺, c⁻) = T (the lower's) with T₁₁ = T₂₂ = (a + b)/2
/// and T₁₂ = T₂₁ = (a − b)/2, solved for the outgoing waves.
fn interface(upper: &Modes, lower: &Modes) -> Result<Scattering> {
    let n = upper.q.len();
    let a = upper.w.partial_piv_lu().solve(&lower.w);
    let b = upper.v.partial_piv_lu().solve(&lower.v);
    let t11 = Mat::<c64>::from_fn(n, n, |i, j| (a[(i, j)] + b[(i, j)]) * 0.5);
    let t12 = Mat::<c64>::from_fn(n, n, |i, j| (a[(i, j)] - b[(i, j)]) * 0.5);
    let lu = t11.partial_piv_lu();
    let s21 = lu.solve(Mat::<c64>::identity(n, n));
    let s22 = -lu.solve(&t12);
    let s11 = &t12 * &s21;
    let s12 = &t11 + &t12 * &s22;
    finite(Scattering { s11, s12, s21, s22 })
}

/// `s` followed by a layer whose modes go as `x` = e^(−qd) across it, each way.
fn propagate(s: Scattering, x: &[c64]) -> Scattering {
    let n = x.len();
    Scattering {
        s11: s.s11,
        s12: Mat::<c64>::from_fn(n, n, |i, j| s.s12[(i, j)] * x[j]),
        s21: Mat::<c64>::from_fn(n, n, |i, j| x[i] * s.s21[(i, j)]),
        s22: Mat::<c64>::from_fn(n, n, |i, j| x[i] * s.s22[(i, j)] * x[j]),
    }
}

/// The Redheffer star product: `a` above `b`.
fn star(a: &Scattering, b: &Scattering) -> Result<Scattering> {
    let n = a.s11.nrows();
    // the waves between: down d = (I − a22 b11)⁻¹ (a21 in + a22 b12 in′), up u = b11 d + b12 in′
    let m = Mat::<c64>::identity(n, n) - &a.s22 * &b.s11;
    let lu = m.partial_piv_lu();
    let x = lu.solve(&a.s21);
    let y = lu.solve(&a.s22 * &b.s12);
    let s11 = &a.s11 + &a.s12 * (&b.s11 * &x);
    let s21 = &b.s21 * &x;
    let s12 = &a.s12 * (&b.s12 + &b.s11 * &y);
    let s22 = &b.s22 + &b.s21 * &y;
    finite(Scattering { s11, s12, s21, s22 })
}

fn finite(s: Scattering) -> Result<Scattering> {
    let ok = |m: &Mat<c64>| {
        (0..m.nrows())
            .all(|i| (0..m.ncols()).all(|j| m[(i, j)].re.is_finite() && m[(i, j)].im.is_finite()))
    };
    if ok(&s.s11) && ok(&s.s12) && ok(&s.s21) && ok(&s.s22) {
        Ok(s)
    } else {
        Err(Error::invalid(
            "diffraction",
            "a scattering matrix isn't finite (a singular interface)",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wl(um: f64) -> Wavelength {
        Wavelength::um(um).unwrap()
    }

    #[test]
    fn a_single_interface_is_fresnel() {
        let g = Grating::new(Length::um(0.5), c64::new(1.0, 0.0), &[], c64::new(1.5, 0.0)).unwrap();
        for pol in [Polarization::Te, Polarization::Tm] {
            let d = g.diffract(pol, wl(1.0), 0.0, 3).unwrap();
            let r = d.r[3];
            let expected = match pol {
                Polarization::Te => (1.0 - 1.5) / (1.0 + 1.5),
                Polarization::Tm => (1.5 - 1.0) / (1.5 + 1.0),
            };
            assert!(portable::complex::abs(r - expected) < 1e-14, "{pol:?}: {r}");
            assert!((d.reflectance + d.transmittance - 1.0).abs() < 1e-14);
        }
    }

    #[test]
    fn a_lossless_grating_conserves_energy() {
        let si = c64::new(3.48, 0.0);
        let air = c64::new(1.0, 0.0);
        let layer = Layer::lamellar(
            &[(si, Length::um(0.3)), (air, Length::um(0.4))],
            Length::um(0.5),
        );
        let g = Grating::new(Length::um(0.7), air, &[layer], c64::new(1.45, 0.0)).unwrap();
        for pol in [Polarization::Te, Polarization::Tm] {
            let d = g.diffract(pol, wl(1.0), 0.2, 15).unwrap();
            assert!(d.absorption.abs() < 1e-12, "{pol:?}: {}", d.absorption);
        }
    }
}
