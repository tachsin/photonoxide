//! Sidewall-roughness loss: the power a guided mode radiates from rough walls, by the
//! volume-current method.
//!
//! A wall displaced by f(z) from the smooth guide's puts a sheet of polarization current
//! J = −iωε₀ (n_core² − n_clad²) E f(z) beside it, E the smooth guide's mode at the wall on the
//! core's side, and the sheet radiates. f is random, with the exponential autocorrelation
//! R(u) = σ² e^(−|u|/L_c) and its spectrum R̃(ξ) = ∫ R(u) e^(iξu) du = 2σ²L_c/(1 + L_c²ξ²)
//! (T. Barwicz, H. A. Haus, J. Lightwave Technol. 23, 2719 (2005),
//! [doi:10.1109/JLT.2005.850816](https://doi.org/10.1109/JLT.2005.850816), Eqs. 1–3; F. P.
//! Payne, J. P. R. Lacey, Opt. Quantum Electron. 26, 977 (1994),
//! [doi:10.1007/BF00708339](https://doi.org/10.1007/BF00708339), Eqs. 2, 3 and 5). Over a
//! length 2L its ensemble-averaged |∫ f(z) e^(i(β − k cos θ)z) dz|² is 2L R̃(β − k cos θ), so a
//! wall radiates in the direction θ from the axis what its spectrum holds at β − k cos θ; the
//! two walls are uncorrelated and their powers add. The loss is the power radiated per length
//! over the mode's power, α, the power falling as e^(−αz); in dB, 10 log₁₀(e) α.
//!
//! - **A slab** ([`slab_loss`]): the TE₀ mode of a symmetric slab, both walls rough, the
//!   sheets radiating into the cladding by the 2D Green's function (i/4) H₀⁽¹⁾(kr), all on the
//!   exact mode ([`super::slab`]): the power per angle from the far field, the mode's power from
//!   ½ Re ∫ E × H*, each by quadrature. Payne and Lacey's closed form for the same slab
//!   ([`payne_lacey`], their Eqs. 8–12) is its check.
//! - **A rectangular core** ([`sidewall_loss`]): Barwicz and Haus's 3D analysis, their
//!   Section V: two rough sidewalls, top and bottom smooth, a uniform cladding. Each wall's
//!   current is a rod the core's height a tall, its strength v from the full-vector mode's field
//!   along the wall, v² = ∫ |E|² over the height, its shape along the height a cosine (or, for
//!   the TM-like mode's E_z, a sine): their Eqs. 8–11. The rod radiates beside a slab of the
//!   core's index and the core's width, the dyadic Green's function of that one-layer medium to
//!   leading order (their Eqs. 12–16: the direct wave, its reflection from the slab on the
//!   source's side and its transmission through it on the other, by the generalized Fresnel
//!   coefficients of their Eq. 14), and the far-field power is integrated over the sphere with
//!   the roughness's array factor (their Eqs. 17–20). The TE-like mode's current is its
//!   component normal to the wall and E_z, the TM-like mode's the vertical one and E_z, weighted
//!   by γ, the power ratio of E_z to the main component at the wall, and the third component's
//!   power shared between them (their Eqs. 19–20).
//!
//! **Payne and Lacey's factor of 2.** Barwicz and Haus (p. 2729) write that a factor of 2 could
//! be missing in Lacey and Payne's 1990 Eq. 16a (Payne and Lacey 1994's Eq. 1) and that an extra
//! one is in Payne and Lacey's Eq. 9, so that Eq. 10 holds. On this module's exact slab mode,
//! both are so ([`slab_loss`]'s tests): the field normalized to ∫ φ² = 1 has φ²(d) at the wall
//! half of Eq. 9's, and the volume current's loss is twice Eq. 1's with that φ²(d), so equal to
//! Eq. 10 as printed. Eq. 1's 1/n₁ is the weakly guiding slab's β ≈ n₁k₀: the mode's power
//! carries β, so the exact loss is Eq. 10 times n₁k₀/β.
//!
//! **The reflected p-wave.** Barwicz and Haus's leading-order G₁,₁ (p. 2727) projects the
//! source on the outgoing wave's vector in both its direct and its reflected TM term, so a
//! current along the wall's plane gets (1 + R̂_TM) where the wave it launches towards the slab,
//! reflected, gives (1 − R̂_TM) with their R^TM (Eq. 14), the H-field coefficient: a dipole
//! at a planar interface radiates so (L. Novotny, B. Hecht, *Principles of Nano-Optics*, 2nd ed.,
//! Cambridge University Press (2012),
//! [doi:10.1017/CBO9780511794193](https://doi.org/10.1017/CBO9780511794193), Ch. 10), and only so
//! do the s and p waves agree along the wall's normal, where they are the same wave. This
//! module takes (1 − R̂_TM); the difference is small beside a low-index core and large beside
//! silicon (docs/methods/devices.md, "Sidewall roughness").
//!
//! **Corners.** The transverse field is singular at the core's corners, |E|² ∝ r^(2τ − 2)
//! ([`corner_exponent`]), integrable, and the finite differences' fields in the two rows of cells
//! nearest a corner don't converge to it; those rows take the singularity's integral, fitted to the\n//! next row.
//! Hadley's equations ([`crate::mode::hadley`]) give the fields beside the corners that converge;
//! the standard scheme's, beside a high-contrast corner, don't.
//!
//! Both are first order in σ and take the smooth guide's mode and β; the 3D analysis takes the
//! field at the wall on the core's side, as Barwicz and Haus do, and its scope is theirs: a
//! rectangular core in a uniform cladding.

use std::f64::consts::PI;

use num_complex::Complex64 as c64;

use super::Polarization;
use super::slab::Slab;
use super::vector::{Boundary, CrossSection, VectorMode};
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

/// dB per neper of power, 10 log₁₀(e).
const DB_PER_NEPER: f64 = 10.0 * std::f64::consts::LOG10_E;

/// µm per cm.
const UM_PER_CM: f64 = 1e4;

/// A wall's roughness: its rms deviation σ and the correlation length L_c of its exponential
/// autocorrelation R(u) = σ² e^(−|u|/L_c) (Barwicz and Haus's Eq. 1; Payne and Lacey's Eq. 5).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Roughness {
    sigma: f64,
    correlation: f64,
}

impl Roughness {
    /// Roughness of rms deviation `sigma` and correlation length `correlation`, its
    /// autocorrelation exponential.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless σ is finite and at least 0 and L_c finite and above 0.
    pub fn exponential(sigma: Length, correlation: Length) -> Result<Roughness> {
        let (s, l) = (sigma.to_um(), correlation.to_um());
        if !(s.is_finite() && s >= 0.0) {
            return Err(Error::invalid(
                "roughness",
                format!("the rms deviation must be at least 0, got {sigma}"),
            ));
        }
        if !(l.is_finite() && l > 0.0) {
            return Err(Error::invalid(
                "roughness",
                format!("the correlation length must be above 0, got {correlation}"),
            ));
        }
        Ok(Roughness {
            sigma: s,
            correlation: l,
        })
    }

    /// The rms deviation σ.
    pub fn sigma(&self) -> Length {
        Length::um(self.sigma)
    }

    /// The correlation length L_c.
    pub fn correlation(&self) -> Length {
        Length::um(self.correlation)
    }

    /// The spectrum R̃(ξ) = 2σ²L_c/(1 + L_c²ξ²), µm³, at ξ in 1/µm (Barwicz and Haus's Eq. 3).
    pub fn spectrum(&self, xi: f64) -> f64 {
        let l = self.correlation;
        2.0 * self.sigma * self.sigma * l / (1.0 + l * l * xi * xi)
    }
}

/// The loss in dB per cm of a power attenuation `alpha` per µm.
pub fn db_per_cm(alpha: f64) -> f64 {
    DB_PER_NEPER * alpha * UM_PER_CM
}

/// A symmetric slab's TE₀ loss from its two rough walls, by the volume current in 2D on the
/// exact mode ([`slab_loss`]).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SlabLoss {
    /// The power attenuation α, per µm: the power falls as e^(−αz).
    pub alpha: f64,
    /// The mode's effective index β/k₀.
    pub effective_index: f64,
    /// φ²(d): the field at a wall squared, the field normalized to ∫ φ² dx = 1, per µm.
    pub field_at_wall: f64,
    /// Payne and Lacey's S = ∫₀^π R̃(β − n_clad k₀ cos θ) dθ (their Eq. 6), µm³, by quadrature.
    pub spectrum_integral: f64,
}

impl SlabLoss {
    /// The loss in dB per cm.
    pub fn db_per_cm(&self) -> f64 {
        db_per_cm(self.alpha)
    }
}

/// The TE₀ mode of a slab of index `core`, `thickness` thick, in `cladding` on both sides: its
/// power attenuation at `wavelength` from both walls rough with `roughness`, independently, by
/// the volume current radiating into the cladding, on the exact mode ([`super::slab`]).
///
/// Each wall's sheet current, −iωε₀ (n_core² − n_clad²) E(wall) f(z), is a line source in 2D:
/// its field −(ωμ₀/4) I H₀⁽¹⁾(kr), k = n_clad k₀, carries |E|² r/(2η) per angle in the far field,
/// |H₀⁽¹⁾|² → 2/(πkr) and η = Z₀/n_clad. The power per length is that, with ⟨|I|²⟩/2L from the
/// spectrum, integrated over the full circle for each wall; the mode's power per unit width,
/// ½ Re ∫ E × H* = (β/2ωμ₀) ∫ |E|² dx. Every integral is a Gauss–Legendre sum on the exact
/// field, the cladding's to 50 decay lengths.
///
/// # Errors
///
/// [`Error::InvalidValue`] for a slab [`Slab::new`] refuses, or one with no guided TE mode.
pub fn slab_loss(
    core: f64,
    cladding: f64,
    thickness: Length,
    wavelength: Wavelength,
    roughness: &Roughness,
) -> Result<SlabLoss> {
    let slab = Slab::new(cladding, core, cladding, thickness)?;
    let Some(mode) = slab.modes(Polarization::Te, wavelength).into_iter().next() else {
        return Err(Error::invalid("slab", "it guides no TE mode"));
    };
    let k0 = wavelength.wavenumber();
    let n = mode.effective_index();
    let beta = n * k0;
    let t = thickness.to_um();
    let (_, q, _) = mode.wavenumbers();
    let e2 = |x: f64| mode.field(Length::um(x)).0.powi(2);
    // ∫ E² dx: the core in panels, each cladding to 50 decay lengths
    let tail = 50.0 / q;
    let norm = integrate(-tail, 0.0, 40, &e2)
        + integrate(0.0, t, 16, &e2)
        + integrate(t, t + tail, 40, &e2);
    let wall = mode.field(Length::ZERO).0;
    // units with Z₀ = 1: ωμ₀ = k₀, ωε₀ = k₀
    let power = beta / (2.0 * k0) * norm;
    let k = cladding * k0;
    let eta = 1.0 / cladding;
    let delta = core * core - cladding * cladding;
    // per angle: r |E|²/(2η), |E|² = (ωμ₀/4)² |I|² 2/(πkr), ⟨|I|²⟩ per length = (ωε₀ Δε E)² R̃
    let per_angle = |theta: f64| {
        let current = (k0 * delta * wall).powi(2) * roughness.spectrum(beta - k * theta.cos());
        (k0 / 4.0).powi(2) * current * 2.0 / (PI * k) / (2.0 * eta)
    };
    let radiated = 2.0 * integrate(0.0, 2.0 * PI, 64, &per_angle);
    let s = integrate(0.0, PI, 64, &|theta: f64| {
        roughness.spectrum(beta - k * theta.cos())
    });
    Ok(SlabLoss {
        alpha: radiated / power,
        effective_index: n,
        field_at_wall: wall * wall / norm,
        spectrum_integral: s,
    })
}

/// Payne and Lacey's closed form for the slab of [`slab_loss`], half-thickness d, with their
/// normalized U = d√(n₁²k₀² − β²), V = k₀d√(n₁² − n₂²), W = d√(β² − n₂²k₀²), β the exact
/// slab's.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PayneLacey {
    /// α by their Eq. 10, σ²/(√2 k₀d⁴n₁) g(V) f_e(x, γ), with g = U²V²/(1 + W) (Eq. 11) and f_e
    /// (Eq. 12), x = W L_c/d, γ = n₂V/(n₁W√Δ), Δ = (n₁² − n₂²)/2n₁²: per µm, as printed.
    pub eq10: f64,
    /// φ²(d) by their Eq. 9, 2V(1 − b)/(d(V + 1/√b)), b = W²/V², per µm.
    pub eq9: f64,
    /// Their S in closed form (Eq. 8), µm³.
    pub eq8: f64,
    /// U, V and W.
    pub uvw: (f64, f64, f64),
}

impl PayneLacey {
    /// Their Eq. 1 for both walls, φ²(d)(n₁² − n₂²)² k₀³ S/(4πn₁), with `field_at_wall` for
    /// φ²(d) and Eq. 8's S, per µm.
    pub fn eq1(&self, field_at_wall: f64, core: f64, cladding: f64, wavelength: Wavelength) -> f64 {
        let k0 = wavelength.wavenumber();
        field_at_wall * (core * core - cladding * cladding).powi(2) * k0.powi(3) * self.eq8
            / (4.0 * PI * core)
    }
}

/// Payne and Lacey's Eqs. 8–12 for the slab of [`slab_loss`] (their Section 2, pp. 979–980): β
/// from the exact slab.
///
/// # Errors
///
/// As [`slab_loss`].
pub fn payne_lacey(
    core: f64,
    cladding: f64,
    thickness: Length,
    wavelength: Wavelength,
    roughness: &Roughness,
) -> Result<PayneLacey> {
    let slab = Slab::new(cladding, core, cladding, thickness)?;
    let Some(mode) = slab.modes(Polarization::Te, wavelength).into_iter().next() else {
        return Err(Error::invalid("slab", "it guides no TE mode"));
    };
    let k0 = wavelength.wavenumber();
    let beta = mode.effective_index() * k0;
    let d = thickness.to_um() / 2.0;
    let (n1, n2) = (core, cladding);
    let u = d * (n1 * n1 * k0 * k0 - beta * beta).sqrt();
    let w = d * (beta * beta - n2 * n2 * k0 * k0).sqrt();
    let v = k0 * d * (n1 * n1 - n2 * n2).sqrt();
    let b = (w / v).powi(2);
    let eq9 = 2.0 * v * (1.0 - b) / (d * (v + 1.0 / b.sqrt()));
    let (sigma, lc) = (roughness.sigma, roughness.correlation);
    // Eq. 8
    let m = 1.0 - lc * lc * (beta * beta - n2 * n2 * k0 * k0);
    let a = (4.0 * beta * beta * lc * lc + m * m).sqrt();
    let eq8 = 2f64.sqrt() * sigma * sigma * lc * PI * (a + m).sqrt() / a;
    // Eqs. 10–12
    let delta = (n1 * n1 - n2 * n2) / (2.0 * n1 * n1);
    let x = w * lc / d;
    let gamma = n2 * v / (n1 * w * delta.sqrt());
    let g = u * u * v * v / (1.0 + w);
    let root = ((1.0 + x * x).powi(2) + 2.0 * x * x * gamma * gamma).sqrt();
    let fe = x * (root + 1.0 - x * x).sqrt() / root;
    let eq10 = sigma * sigma / (2f64.sqrt() * k0 * d.powi(4) * n1) * g * fe;
    Ok(PayneLacey {
        eq10,
        eq9,
        eq8,
        uvw: (u, v, w),
    })
}

/// A rectangular guide's loss from its two rough sidewalls ([`sidewall_loss`]).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SidewallLoss {
    /// The power attenuation α, per µm.
    pub alpha: f64,
    /// Whether the mode is TE-like, its field at the walls mostly normal to them.
    pub te_like: bool,
    /// γ: the power of E_z along a wall over its main component's (Barwicz and Haus's γ_zy for
    /// the TE-like mode, γ_zx for the TM-like).
    pub gamma: f64,
    /// v²/P: ∫ |E|² along one wall's height over the mode's power, 1/µm (in the solver's units,
    /// E in Z₀ times H's).
    pub wall_field: f64,
    /// The core's width and height, µm.
    pub size: (f64, f64),
    /// The core's and the cladding's indices.
    pub indices: (f64, f64),
}

impl SidewallLoss {
    /// The loss in dB per cm.
    pub fn db_per_cm(&self) -> f64 {
        db_per_cm(self.alpha)
    }
}

/// The loss of `mode`, found on `cs` at `wavelength`, from both sidewalls rough with
/// `roughness`, by Barwicz and Haus's 3D volume-current analysis (the module's).
///
/// The core is the cells of the highest permittivity, which must fill a rectangle; every other
/// cell is the cladding, uniform. The rough wall is the core's east side (largest x), and the
/// field there is the mode's ([`VectorMode::fields`], each component's normal derivative
/// one-sided beside an interface) on the core's side, extrapolated to the wall linearly from
/// the two columns of cells inside it, the rows at a corner from its singularity (the module's).
/// The mode is TE-like when H_y holds most of its transverse H ([`VectorMode::te_fraction`]). A
/// mirror wall at the window's west or south edge through the core is a plane of symmetry: the
/// core and the mode are mirrored across it. Give it a mode by Hadley's equations
/// ([`crate::mode::hadley::modes`]) for a high-contrast core: the standard scheme's fields
/// beside its corners don't converge.
///
/// # Errors
///
/// [`Error::InvalidValue`] for a mode not found on `cs`, a core that isn't a rectangle at least
/// two cells wide with cladding beyond its east side, a cladding that isn't uniform, or anything
/// but [`Boundary::Zero`] on the east edge.
pub fn sidewall_loss(
    cs: &CrossSection,
    mode: &VectorMode,
    wavelength: Wavelength,
    roughness: &Roughness,
) -> Result<SidewallLoss> {
    let core = Core::of(cs)?;
    let fields = mode.fields_sided(cs, true)?;
    let sums = wall_sums(cs, &core, &fields, CORNER_CELLS)?;
    let mirrored_rows = f64::from(u8::from(core.south_mirror) + u8::from(core.north_mirror));
    let rows = 2f64.powf(mirrored_rows);
    let columns = if core.west_mirror { 2.0 } else { 1.0 };
    let power = fields.power() * rows * columns;
    if !(power.is_finite() && power > 0.0) {
        return Err(Error::invalid(
            "sidewall roughness",
            format!("the mode carries no power through the cross-section ({power})"),
        ));
    }
    let sums = sums.map(|s| s * rows);
    let te_like = mode.te_fraction() >= 0.5;
    let beta = mode.effective_index().re * wavelength.wavenumber();
    Ok(loss_of(
        &core,
        sums.map(|s| s / power),
        te_like,
        beta,
        wavelength,
        roughness,
    ))
}

/// The loss from the wall's ∫ |E|² over the mode's power, `sums` (normal, vertical, along).
pub(crate) fn loss_of(
    core: &Core,
    sums: [f64; 3],
    te_like: bool,
    beta: f64,
    wavelength: Wavelength,
    roughness: &Roughness,
) -> SidewallLoss {
    let v2 = sums.iter().sum::<f64>();
    let k0 = wavelength.wavenumber();
    let radiation = Radiation {
        k0,
        cladding: core.cladding.sqrt(),
        eps_core: core.core,
        eps_cladding: core.cladding,
        width: Some(core.width),
        height: core.height,
        beta,
        roughness: *roughness,
        printed: false,
    };
    // the rod's currents, in (normal, vertical, along) axes, and their shapes
    let (main, gamma, shapes) = if te_like {
        (
            [1.0, 0.0, 0.0],
            sums[2] / sums[0],
            (Shape::Cosine, Shape::Cosine),
        )
    } else {
        (
            [0.0, 1.0, 0.0],
            sums[2] / sums[1],
            (Shape::Cosine, Shape::Sine),
        )
    };
    let pattern = (radiation.integral(main, shapes.0)
        + gamma * radiation.integral([0.0, 0.0, 1.0], shapes.1))
        / (1.0 + gamma);
    let delta = core.core - core.cladding;
    let alpha = radiation.prefactor(delta, v2) * pattern;
    SidewallLoss {
        alpha,
        te_like,
        gamma,
        wall_field: v2,
        size: (core.width, core.height),
        indices: (core.core.sqrt(), core.cladding.sqrt()),
    }
}

/// The rows of cells at each corner of the wall whose transverse field is taken from the
/// corner's singularity instead of the grid.
pub(crate) const CORNER_CELLS: usize = 2;

/// The exponent τ of the field at a right-angled corner of a core of permittivity `core` in
/// `cladding`, E ∝ r^(τ − 1): the smallest root of ε_core tan(τπ/4) = −ε_clad tan(3τπ/4), the
/// quasi-static field of a dielectric wedge a quarter turn wide, symmetric about its bisector
/// (J. Meixner, IEEE Trans. Antennas Propag. 20, 442 (1972),
/// [doi:10.1109/TAP.1972.1140243](https://doi.org/10.1109/TAP.1972.1140243)): from 2/3 for a
/// contrast without bound to 1 for none.
pub fn corner_exponent(core: f64, cladding: f64) -> f64 {
    if core <= cladding {
        return 1.0;
    }
    // t = τπ/4 in (π/6, π/4): the left side is −∞ at π/6, ε_core − ε_clad > 0 at π/4
    let f = |t: f64| core * t.tan() + cladding * (3.0 * t).tan();
    let (mut a, mut b) = (PI / 6.0 + 1e-15, PI / 4.0);
    for _ in 0..200 {
        let m = 0.5 * (a + b);
        if f(m) < 0.0 {
            a = m;
        } else {
            b = m;
        }
    }
    2.0 * (a + b) / PI
}

/// ∫ |E|² along the wall's height on the core's side, each component (normal, vertical, along
/// the guide), over the rows the cross-section holds. E_z is continuous and bounded at the
/// corners and is summed over every row. The transverse components are singular at a corner,
/// |E|² ∝ r^(2τ − 2) ([`corner_exponent`]), and the finite differences' fields in the `corner`
/// rows of cells nearest it don't converge to it: those rows take ∫₀^d A r^(2τ − 2) dr =
/// A d^(2τ − 1)/(2τ − 1), d their height, A from the next row's value at its centre.
pub(crate) fn wall_sums(
    cs: &CrossSection,
    core: &Core,
    fields: &crate::mode::fields::Fields,
    corner: usize,
) -> Result<[f64; 3]> {
    let rows = wall_field(cs, core, fields)?;
    let y = cs.y();
    let tau = corner_exponent(core.core, core.cladding);
    let p = 2.0 * tau - 1.0;
    // the corners in the window: the core's ends along y not on a mirror
    let mut corners = Vec::new();
    if !core.south_mirror {
        corners.push((y[core.rows.0], 0usize, 1isize));
    }
    if !core.north_mirror {
        corners.push((y[core.rows.1], rows.len() - 1, -1isize));
    }
    let near = |k: usize| {
        corners.iter().any(|&(_, end, step)| {
            let from = if step > 0 { k } else { end - k };
            from < corner
        })
    };
    if corners.len() * corner + 1 > rows.len() {
        return Err(Error::invalid(
            "sidewall roughness",
            format!(
                "the wall must be more than {} cells high",
                corners.len() * corner
            ),
        ));
    }
    let mut sums = [0.0; 3];
    for (k, (dy, _, e)) in rows.iter().enumerate() {
        sums[2] += e[2].norm_sqr() * dy;
        if !near(k) {
            for c in 0..2 {
                sums[c] += e[c].norm_sqr() * dy;
            }
        }
    }
    for &(at, end, step) in &corners {
        let first = (end as isize + step * corner as isize) as usize;
        let (_, centre, e) = rows[first];
        let r = (centre - at).abs();
        let d = (y[core.rows.0 + if step > 0 { first } else { first + 1 }] - at).abs();
        for c in 0..2 {
            let a = e[c].norm_sqr() * r.powf(2.0 - 2.0 * tau);
            sums[c] += a * d.powf(p) / p;
        }
    }
    Ok(sums)
}

/// The field at the core's east wall, on the core's side, in each row of cells along it: the
/// row's height, its centre, and E there, extrapolated linearly from the two columns of cells
/// inside the wall.
pub(crate) fn wall_field(
    cs: &CrossSection,
    core: &Core,
    fields: &crate::mode::fields::Fields,
) -> Result<Vec<(f64, f64, [c64; 3])>> {
    let (x, y) = (cs.x(), cs.y());
    let i1 = core.columns.1 - 1;
    if i1 < core.columns.0 + 1 {
        return Err(Error::invalid(
            "sidewall roughness",
            "the core must be at least two cells wide beside its wall",
        ));
    }
    let i2 = i1 - 1;
    let xc = fields.x();
    let along = (x[core.columns.1] - xc[i1]) / (xc[i1] - xc[i2]);
    Ok((core.rows.0..core.rows.1)
        .map(|j| {
            let (e1, e2) = (fields.e(i1, j), fields.e(i2, j));
            let e = [0, 1, 2].map(|c| e1[c] + (e1[c] - e2[c]) * along);
            (y[j + 1] - y[j], fields.y()[j], e)
        })
        .collect())
}

/// A cross-section's rectangular core in a uniform cladding.
pub(crate) struct Core {
    /// The core's cells: columns and rows, from and to (exclusive).
    columns: (usize, usize),
    rows: (usize, usize),
    /// Relative permittivities.
    core: f64,
    cladding: f64,
    /// Its full width and height, µm, mirrors unfolded.
    width: f64,
    height: f64,
    west_mirror: bool,
    south_mirror: bool,
    north_mirror: bool,
}

impl Core {
    fn of(cs: &CrossSection) -> Result<Core> {
        let bad = |why: &str| Error::invalid("sidewall roughness", why.to_owned());
        let (x, y) = (cs.x(), cs.y());
        let (nx, ny) = (x.len() - 1, y.len() - 1);
        let eps = |i: usize, j: usize| cs.cells()[i * ny + j].xx;
        let core = (0..nx)
            .flat_map(|i| (0..ny).map(move |j| (i, j)))
            .map(|(i, j)| eps(i, j).re)
            .fold(f64::NEG_INFINITY, f64::max);
        let is_core = |i: usize, j: usize| eps(i, j) == c64::new(core, 0.0);
        let (mut i0, mut i1, mut j0, mut j1) = (nx, 0, ny, 0);
        for i in 0..nx {
            for j in 0..ny {
                if is_core(i, j) {
                    (i0, i1, j0, j1) = (i0.min(i), i1.max(i + 1), j0.min(j), j1.max(j + 1));
                }
            }
        }
        let mut cladding = None;
        for i in 0..nx {
            for j in 0..ny {
                let inside = (i0..i1).contains(&i) && (j0..j1).contains(&j);
                if inside != is_core(i, j) {
                    return Err(bad("the core must be a rectangle of one lossless medium"));
                }
                if !inside {
                    let e = eps(i, j);
                    if e.im != 0.0 || cladding.is_some_and(|c| c != e.re) {
                        return Err(bad(
                            "the cladding must be uniform and lossless: Barwicz and Haus's \
                             analysis is for a rectangular core in one medium",
                        ));
                    }
                    cladding = Some(e.re);
                }
            }
        }
        let Some(cladding) = cladding else {
            return Err(bad("the window holds no cladding"));
        };
        if i1 >= nx {
            return Err(bad(
                "the window must hold cladding beyond the core's east wall",
            ));
        }
        let b = cs.boundaries();
        if b.east != Boundary::Zero {
            return Err(bad(
                "the east edge, beyond the rough wall, must be open (Boundary::Zero)",
            ));
        }
        let mirror = |m: Boundary| m != Boundary::Zero;
        let west_mirror = mirror(b.west) && i0 == 0;
        let south_mirror = mirror(b.south) && j0 == 0;
        let north_mirror = mirror(b.north) && j1 == ny;
        let width = (x[i1] - x[i0]) * if west_mirror { 2.0 } else { 1.0 };
        let height = (y[j1] - y[j0])
            * (1.0 + f64::from(u8::from(south_mirror)) + f64::from(u8::from(north_mirror)));
        Ok(Core {
            columns: (i0, i1),
            rows: (j0, j1),
            core,
            cladding,
            width,
            height,
            west_mirror,
            south_mirror,
            north_mirror,
        })
    }
}

/// The shape of a rod's current along its height a, normalized to ∫ Υ² = 1 (Barwicz and Haus's
/// vertical shape functions, Fig. 4(d)).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Shape {
    /// 1/√a: the tests' check of the 3D analysis against the 2D one.
    #[cfg_attr(not(test), allow(dead_code))]
    Square,
    /// √(2/a) cos(πx/a).
    Cosine,
    /// √(2/a) sin(2πx/a).
    Sine,
}

impl Shape {
    /// |F(k)|² = |∫ Υ(x) e^(−ikx) dx|², µm: a sinc² for the square, and Barwicz and Haus's
    /// Eqs. 10 and 11 for the cosine and the sine, written about their removable poles at
    /// |k|a = π and 2π as sin(δa/2)/(δ(2p + δ)), δ = |k| − p.
    pub(crate) fn power(self, a: f64, k: f64) -> f64 {
        let k = k.abs();
        let pole = |p: f64| {
            let d = k - p;
            let half = 0.5 * d * a;
            let sinc = if half.abs() < 1e-8 {
                1.0 - half * half / 6.0
            } else {
                half.sin() / half
            };
            0.5 * a * sinc / (2.0 * p + d)
        };
        match self {
            Shape::Square => {
                let half = 0.5 * k * a;
                let sinc = if half < 1e-8 { 1.0 } else { half.sin() / half };
                a * sinc * sinc
            }
            Shape::Cosine => {
                let p = PI / a;
                (2.0 / a) * (2.0 * p * pole(p)).powi(2)
            }
            Shape::Sine => {
                let q = 2.0 * PI / a;
                (2.0 / a) * (2.0 * q * pole(q)).powi(2)
            }
        }
    }
}

/// A rod of current at a wall, radiating beside the slab of the core (or in the cladding alone):
/// Barwicz and Haus's Sections IV and V.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Radiation {
    /// k₀, 1/µm.
    pub(crate) k0: f64,
    /// The cladding's index.
    pub(crate) cladding: f64,
    /// The core's and the cladding's relative permittivities.
    pub(crate) eps_core: f64,
    pub(crate) eps_cladding: f64,
    /// The core's width, the slab the rod radiates beside; `None` radiates in the cladding alone
    /// (their Section IV).
    pub(crate) width: Option<f64>,
    /// The rod's height a, µm.
    pub(crate) height: f64,
    /// The mode's β, 1/µm.
    pub(crate) beta: f64,
    pub(crate) roughness: Roughness,
    /// Barwicz and Haus's G₁,₁ as printed, the reflected p-wave projected on the outgoing wave's
    /// vector, (1 + R̂_TM) for a current along the wall's plane: only to measure its difference
    /// (the module's).
    pub(crate) printed: bool,
}

impl Radiation {
    /// α over the pattern's integral: k₀⁴ Δε² n_clad (v²/P) / (32π²). A point current J radiates
    /// iωμ₀ e^(ikr)/(4πr) J⊥, so r²|E|²/(2η) = (ωμ₀)² |J⊥|² n_clad/(32π² Z₀); J = ωε₀ Δε v, and
    /// ωμ₀ ωε₀ = k₀².
    pub(crate) fn prefactor(&self, delta_eps: f64, field: f64) -> f64 {
        self.k0.powi(4) * delta_eps * delta_eps * self.cladding * field / (32.0 * PI * PI)
    }

    /// The far-field |a|² of a unit current along `j` (axes: the wall's normal, out of the core;
    /// the vertical; the guide's axis) in the direction `r`: |J⊥|² in the cladding alone; beside
    /// the slab, the s and p waves each with the slab's generalized reflection (on the source's
    /// side) or transmission (beyond it), the source on its face (Barwicz and Haus's Eq. 14 and
    /// their G₁,₁ and G₁,₃).
    pub(crate) fn far(&self, j: [f64; 3], r: [f64; 3]) -> f64 {
        let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let cross = |a: [f64; 3], b: [f64; 3]| {
            [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ]
        };
        let Some(b) = self.width else {
            let along = dot(r, j);
            return dot(j, j) - along * along;
        };
        let t = r[1].hypot(r[2]);
        // s: normal to the plane of the wall's normal and r
        let s = if t > 0.0 {
            [0.0, -r[2] / t, r[1] / t]
        } else {
            [0.0, 1.0, 0.0]
        };
        let k1 = self.cladding * self.k0;
        let k1z = k1 * r[0].abs();
        let k2z = (self.eps_core * self.k0 * self.k0 - k1 * k1 * t * t).sqrt();
        let phase = c64::from_polar(1.0, k2z * b);
        let e = phase * phase;
        // the slab's generalized coefficients from its single interface's R
        let slab = |r12: f64| {
            let denominator = 1.0 - r12 * r12 * e;
            (
                r12 * (1.0 - e) / denominator,
                (1.0 - r12 * r12) * phase / denominator,
            )
        };
        let te = slab((k1z - k2z) / (k1z + k2z));
        let (ec, el) = (self.eps_core, self.eps_cladding);
        let tm = slab((ec * k1z - el * k2z) / (ec * k1z + el * k2z));
        let js = dot(s, j);
        if r[0] >= 0.0 {
            let down = [-r[0], r[1], r[2]];
            let a_s = js * (1.0 + te.0);
            let a_p = if self.printed {
                (1.0 + tm.0) * dot(cross(s, r), j)
            } else {
                dot(cross(s, r), j) + tm.0 * dot(cross(s, down), j)
            };
            a_s.norm_sqr() + a_p.norm_sqr()
        } else {
            let a_s = js * te.1;
            let a_p = tm.1 * dot(cross(s, r), j);
            a_s.norm_sqr() + a_p.norm_sqr()
        }
    }

    /// ∫ dΩ |a(r)|² |F(k_v)|² 2R̃(β − n_clad k₀ r_z) over the sphere, the 2 for the two walls
    /// (Barwicz and Haus's Eqs. 17–18), k_v = n_clad k₀ r_v along the rod: Gauss–Legendre in
    /// r_v (even, so over [0, 1] twice) and the azimuth about it, in panels a lobe of F wide and
    /// a quarter turn wide, each mapped by t²(3 − 2t) so the square roots at grazing angles to
    /// the slab are smooth.
    pub(crate) fn integral(&self, j: [f64; 3], shape: Shape) -> f64 {
        self.integral_with(j, shape, 24, 48)
    }

    pub(crate) fn integral_with(&self, j: [f64; 3], shape: Shape, nu: usize, npsi: usize) -> f64 {
        let k1 = self.cladding * self.k0;
        let lobes = (self.height * k1 / PI).ceil().max(4.0) as usize;
        let (xu, wu) = gauss_legendre(nu);
        let (xp, wp) = gauss_legendre(npsi);
        let mut total = 0.0;
        for pu in 0..lobes {
            let (u0, u1) = (pu as f64 / lobes as f64, (pu + 1) as f64 / lobes as f64);
            for (&tu, &wtu) in xu.iter().zip(&wu) {
                let (u, du) = smooth(u0, u1, tu, wtu);
                let s = (1.0 - u * u).max(0.0).sqrt();
                let f = shape.power(self.height, k1 * u);
                for quarter in 0..4 {
                    let (p0, p1) = (quarter as f64 * PI / 2.0, (quarter + 1) as f64 * PI / 2.0);
                    for (&tp, &wtp) in xp.iter().zip(&wp) {
                        let (psi, dpsi) = smooth(p0, p1, tp, wtp);
                        let r = [s * psi.cos(), u, s * psi.sin()];
                        let spectrum = 2.0 * self.roughness.spectrum(self.beta - k1 * r[2]);
                        total += du * dpsi * self.far(j, r) * f * spectrum;
                    }
                }
            }
        }
        // r_v and −r_v alike
        2.0 * total
    }
}

/// The node and weight on [a, b] of the Gauss–Legendre node `t` (on [−1, 1]) of weight `w`,
/// through the map s = τ²(3 − 2τ), τ = (t + 1)/2, whose derivative vanishes at both ends.
fn smooth(a: f64, b: f64, t: f64, w: f64) -> (f64, f64) {
    let tau = 0.5 * (t + 1.0);
    let s = tau * tau * (3.0 - 2.0 * tau);
    let ds = 6.0 * tau * (1.0 - tau);
    (a + (b - a) * s, (b - a) * ds * 0.5 * w)
}

/// The n-point Gauss–Legendre rule on [−1, 1]: nodes and weights, by Newton's iteration on
/// P_n from Chebyshev's guesses.
pub(crate) fn gauss_legendre(n: usize) -> (Vec<f64>, Vec<f64>) {
    let mut nodes = vec![0.0; n];
    let mut weights = vec![0.0; n];
    for i in 0..n.div_ceil(2) {
        let mut x = (PI * (i as f64 + 0.75) / (n as f64 + 0.5)).cos();
        let mut dp = 0.0;
        for _ in 0..100 {
            // P_n(x) and P_n'(x) by the recurrence
            let (mut p0, mut p1) = (1.0, x);
            for m in 2..=n {
                let p2 = ((2 * m - 1) as f64 * x * p1 - (m - 1) as f64 * p0) / m as f64;
                (p0, p1) = (p1, p2);
            }
            // P_n is p1 and P_(n−1) p0 (1 and x for n = 1)
            dp = n as f64 * (x * p1 - p0) / (x * x - 1.0);
            let step = p1 / dp;
            x -= step;
            if step.abs() < 1e-16 {
                break;
            }
        }
        let w = 2.0 / ((1.0 - x * x) * dp * dp);
        nodes[i] = -x;
        nodes[n - 1 - i] = x;
        weights[i] = w;
        weights[n - 1 - i] = w;
    }
    (nodes, weights)
}

/// ∫ f over [a, b] in `panels` equal panels of 20-point Gauss–Legendre.
fn integrate(a: f64, b: f64, panels: usize, f: &dyn Fn(f64) -> f64) -> f64 {
    let (x, w) = gauss_legendre(20);
    let h = (b - a) / panels as f64;
    (0..panels)
        .map(|p| {
            let mid = a + h * (p as f64 + 0.5);
            x.iter()
                .zip(&w)
                .map(|(&t, &wt)| 0.5 * h * wt * f(mid + 0.5 * h * t))
                .sum::<f64>()
        })
        .sum()
}

#[cfg(test)]
#[path = "roughness_tests.rs"]
mod tests;
