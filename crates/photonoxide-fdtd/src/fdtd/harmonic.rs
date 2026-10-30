//! Harmonic inversion: a signal's frequencies, decay rates and amplitudes, by the filter
//! diagonalization of V. A. Mandelshtam and H. S. Taylor, J. Chem. Phys. 107, 6756 (1997),
//! doi:10.1063/1.475324 (Section II and its summary of the algorithm).
//!
//! The signal cₙ = Σₖ dₖ e^(−inτωₖ) (their Eq. 2), sampled every τ, is the correlation function
//! (Φ₀, Ûⁿ Φ₀) of an evolution operator Û whose eigenvalues are uₖ = e^(−iτωₖ) (Eqs. 4 to 6).
//! In the basis Ψ(z) = Σₙ₌₀ᴹ (Û/z)ⁿ Φ₀ for a few points zⱼ = e^(−iφⱼ) on the unit circle in the
//! window (Eq. 19), Û's matrices U⁽ᵖ⁾(z, z′) = (Ψ(z), Ûᵖ Ψ(z′)) are sums of the signal alone
//! (Eq. 25), and U⁽¹⁾B = u U⁽⁰⁾B (Eq. 23) gives the uₖ near the window. The basis is dominated by
//! the eigenvectors whose uₖ are near the zⱼ (Eq. 21), so a window holding a few frequencies is
//! a small problem whatever the signal's length, and two frequencies closer than the Fourier
//! transform's resolution 2π/(Nτ) are still told apart (their Section II C).

use faer::Mat;
use num_complex::Complex64 as c64;

use super::invalid;
use crate::Result;
use crate::units::Frequency;

/// One term of a signal Σₖ dₖ e^(−iωₖt): a frequency, its decay rate and its amplitude.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Resonance {
    /// Re ωₖ / 2π, c/µm.
    pub frequency: f64,
    /// γₖ = −Im ωₖ, per µm/c: the term's amplitude goes as e^(−γt). Negative for a growing
    /// term.
    pub decay: f64,
    /// dₖ, at the first sample's time. A real signal A e^(−γt) cos(ωt − φ) has the term
    /// ½A e^(iφ) at ω (and its conjugate at −ω, outside a positive window).
    pub amplitude: c64,
    /// ‖(U⁽²⁾ − uₖ²U⁽⁰⁾)Bₖ‖ / ‖uₖ²U⁽⁰⁾Bₖ‖, the step 5 criterion of Mandelshtam and Taylor's
    /// summary: near round-off for a term the signal holds, large for a spurious one.
    pub error: f64,
}

impl Resonance {
    /// The quality factor Q = ω/(2γ), the number of radians the energy takes to fall by e.
    pub fn q(&self) -> f64 {
        std::f64::consts::PI * self.frequency / self.decay
    }
}

/// The terms of `signal` (sampled every `dt`, µm/c) whose frequencies are between `low` and
/// `high`, by filter diagonalization, sorted by frequency.
///
/// The window is split into J = ⌈N τ(ω_max − ω_min)/4π⌉ points, at least 8, as the summary's
/// step 2 suggests, and the basis is Ψ(zⱼ) with M = ⌊(N − 3)/2⌋ (step 3). The generalized
/// eigenproblem is solved on U⁽⁰⁾'s singular vectors above 1e-11 of its largest singular
/// value (step 4), and the amplitudes come from the whole signal (Eq. 27). Every eigenvalue
/// whose frequency is in the window is returned with its error (step 5): what to keep is the
/// caller's choice. A real signal's terms come in pairs at ±ω; a positive window holds one of
/// each pair.
///
/// # Errors
///
/// [`crate::Error::InvalidValue`] if `dt` isn't positive and finite, if `high` isn't above
/// `low` or is at or beyond the Nyquist frequency 1/(2 dt), if the signal has fewer than 8
/// samples or a value that isn't finite, or if the small eigenproblem fails.
pub fn harmonic_inversion(
    signal: &[c64],
    dt: f64,
    low: Frequency,
    high: Frequency,
) -> Result<Vec<Resonance>> {
    if !(dt.is_finite() && dt > 0.0) {
        return Err(invalid(format!(
            "harmonic inversion needs a positive, finite time step, got {dt}"
        )));
    }
    let (f_low, f_high) = (low.to_natural(), high.to_natural());
    if f_high <= f_low || f_high * dt >= 0.5 {
        return Err(invalid(format!(
            "harmonic inversion's window must be from low to high below the Nyquist frequency \
             {} c/µm, got {f_low} to {f_high}",
            0.5 / dt
        )));
    }
    let n = signal.len();
    if n < 8
        || signal
            .iter()
            .any(|c| !(c.re.is_finite() && c.im.is_finite()))
    {
        return Err(invalid(format!(
            "harmonic inversion needs at least 8 finite samples, got {n}"
        )));
    }
    let tau = dt;
    let m = (n - 3) / 2;
    let (phi_low, phi_high) = (
        std::f64::consts::TAU * f_low * tau,
        std::f64::consts::TAU * f_high * tau,
    );
    let j =
        ((n as f64 * (phi_high - phi_low) / (4.0 * std::f64::consts::PI)).ceil() as usize).max(8);
    let z: Vec<c64> = (0..j)
        .map(|q| {
            let phi = phi_low + (q as f64 + 0.5) * (phi_high - phi_low) / j as f64;
            c64::new(0.0, -phi).exp()
        })
        .collect();
    let u: [Mat<c64>; 3] =
        std::array::from_fn(|p| Mat::from_fn(j, j, |a, b| element(signal, m, p, z[a], z[b])));
    let svd = u[0]
        .svd()
        .map_err(|e| invalid(format!("harmonic inversion's SVD failed: {e:?}")))?;
    let s = svd.S().column_vector();
    // the singular values, real, in complex numbers
    let rank = (0..s.nrows())
        .filter(|&k| s[k].re > 1e-11 * s[0].re)
        .count();
    if rank == 0 {
        return Ok(Vec::new());
    }
    let (w, v) = (svd.U(), svd.V());
    let wr = Mat::<c64>::from_fn(j, rank, |a, b| w[(a, b)]);
    let vr = Mat::<c64>::from_fn(j, rank, |a, b| v[(a, b)]);
    // U⁽¹⁾ V y = u U⁽⁰⁾ V y = u W S y, so S⁻¹ Wᴴ U⁽¹⁾ V y = u y
    let mut reduced = wr.adjoint() * &u[1] * &vr;
    for a in 0..rank {
        for b in 0..rank {
            reduced[(a, b)] /= s[a].re;
        }
    }
    let eigen = reduced
        .eigen()
        .map_err(|e| invalid(format!("harmonic inversion's eigenproblem failed: {e:?}")))?;
    let (values, vectors) = (eigen.S().column_vector(), eigen.U());
    let mut found = Vec::new();
    for k in 0..rank {
        let uk = values[k];
        if uk.norm() == 0.0 {
            continue;
        }
        let omega = c64::new(-uk.arg(), uk.norm().ln()) / tau;
        let frequency = omega.re / std::f64::consts::TAU;
        if !(f_low..=f_high).contains(&frequency) {
            continue;
        }
        let y = Mat::<c64>::from_fn(rank, 1, |a, _| vectors[(a, k)]);
        let mut b = &vr * y;
        // normalized as (Υₖ, Υₖ) = Bᵀ U⁽⁰⁾ B = 1, the inner product without conjugation
        let u0b = &u[0] * &b;
        let norm: c64 = (0..j).map(|a| b[(a, 0)] * u0b[(a, 0)]).sum();
        let scale = norm.sqrt().inv();
        for a in 0..j {
            b[(a, 0)] *= scale;
        }
        let u0b = &u[0] * &b;
        let u2b = &u[2] * &b;
        let uk2 = uk * uk;
        let residual: f64 = (0..j)
            .map(|a| (u2b[(a, 0)] - uk2 * u0b[(a, 0)]).norm_sqr())
            .sum::<f64>()
            .sqrt();
        let size: f64 = (0..j)
            .map(|a| (uk2 * u0b[(a, 0)]).norm_sqr())
            .sum::<f64>()
            .sqrt();
        // Eq. 27: d = ((1/(M + 1)) Σⱼ Bⱼ U⁽⁰⁾(zⱼ, uₖ))²
        let sum: c64 = (0..j)
            .map(|a| b[(a, 0)] * element(signal, m, 0, z[a], uk))
            .sum();
        let root = sum / (m + 1) as f64;
        found.push(Resonance {
            frequency,
            decay: -omega.im,
            amplitude: root * root,
            error: residual / size,
        });
    }
    found.sort_by(|a, b| a.frequency.total_cmp(&b.frequency));
    Ok(found)
}

/// U⁽ᵖ⁾(z, z′) = Σₙ,ₙ′₌₀ᴹ z⁻ⁿ z′⁻ⁿ′ c₍ₙ₊ₙ′₊ₚ₎, by Mandelshtam and Taylor's Eq. 25: for z ≠ z′,
///
/// [z f(z′) − z′ f(z) − z⁻ᴹ g(z′) + z′⁻ᴹ g(z)] / (z − z′),
///
/// with f(z) = Σₗ₌₀ᴹ c₍ₗ₊ₚ₎ z⁻ˡ and g(z) = Σₗ₌ᴹ₊₁²ᴹ c₍ₗ₊ₚ₎ zᴹ⁻ˡ⁺¹, and on the diagonal
/// Σₗ₌₀²ᴹ (M − |M − l| + 1) c₍ₗ₊ₚ₎ z⁻ˡ. Both are checked against the double sum.
pub(crate) fn element(c: &[c64], m: usize, p: usize, z: c64, w: c64) -> c64 {
    let (zi, wi) = (z.inv(), w.inv());
    if (z - w).norm() <= 1e-13 * z.norm().max(w.norm()) {
        let mut total = c64::new(0.0, 0.0);
        let mut power = c64::new(1.0, 0.0);
        for l in 0..=2 * m {
            let weight = (m + 1 - l.abs_diff(m)) as f64;
            total += weight * c[l + p] * power;
            power *= zi;
        }
        return total;
    }
    // f(z) and f(w): Σₗ₌₀ᴹ c₍ₗ₊ₚ₎ z⁻ˡ
    let (mut fz, mut fw) = (c64::new(0.0, 0.0), c64::new(0.0, 0.0));
    let (mut pz, mut pw) = (c64::new(1.0, 0.0), c64::new(1.0, 0.0));
    for l in 0..=m {
        fz += c[l + p] * pz;
        fw += c[l + p] * pw;
        pz *= zi;
        pw *= wi;
    }
    // pz = z⁻⁽ᴹ⁺¹⁾ now; g(z) = Σₗ₌ᴹ₊₁²ᴹ c₍ₗ₊ₚ₎ z⁻⁽ˡ⁻ᴹ⁻¹⁾
    let (zm, wm) = (pz * z, pw * w);
    let (mut gz, mut gw) = (c64::new(0.0, 0.0), c64::new(0.0, 0.0));
    let (mut qz, mut qw) = (c64::new(1.0, 0.0), c64::new(1.0, 0.0));
    for l in m + 1..=2 * m {
        gz += c[l + p] * qz;
        gw += c[l + p] * qw;
        qz *= zi;
        qw *= wi;
    }
    (z * fw - w * fz - zm * gw + wm * gz) / (z - w)
}
