//! A beam launched into the expansion ([`super::Eme::launch`]): Gaussians tilted by an angle on
//! a step between uniform media and through a film, against the angular spectrum of Fresnel's
//! and Airy's coefficients, and a Gaussian's overlap with a silicon slab's mode against the
//! exact slab's.

use num_complex::Complex64 as c64;

use super::scattering::GL20;
use super::{Beam, Eme, Section, Slice, power};
use crate::Result;
use crate::mode::Polarization;
use crate::mode::slab::Slab;
use crate::units::{Length, Wavelength};
use crate::validation::{Case, Outcome, Tier};

/// The wavelength, µm.
const LAMBDA: f64 = 1.55;
/// The medium the beam comes from, and the one it meets: oxide and a nitride-like 2.0.
const N1: f64 = 1.444;
const N2: f64 = 2.0;
/// The window, µm, between electric walls: the beam's 9 waists inside it.
const WINDOW: f64 = 40.0;
/// The beam's waist, µm.
const WAIST: f64 = 3.0;
/// The modes each section keeps: every propagating one of the window (about 75) and more.
const MODES: usize = 110;
/// The film's thickness, µm.
const FILM: f64 = 0.3;
/// The angles, degrees.
pub(crate) const ANGLES: [f64; 4] = [0.0, 10.0, 20.0, 30.0];

fn k() -> f64 {
    2.0 * std::f64::consts::PI / LAMBDA
}

fn uniform(n: f64, width: f64) -> Result<Slice> {
    Slice::new(&[(c64::new(n, 0.0), Length::um(width))])
}

/// ∫ g(φ) dφ over (−π/2, π/2) by 20-point Gauss–Legendre panels, `panels` of them.
fn integrate(g: &dyn Fn(f64) -> f64, panels: usize) -> f64 {
    let (lo, hi) = (-0.5 * std::f64::consts::PI, 0.5 * std::f64::consts::PI);
    let h = (hi - lo) / panels as f64;
    let mut sum = 0.0;
    for p in 0..panels {
        let centre = lo + (p as f64 + 0.5) * h;
        for &(node, weight) in &GL20 {
            sum += 0.5 * h * weight * (g(centre - 0.5 * h * node) + g(centre + 0.5 * h * node));
        }
    }
    sum
}

/// The share of a beam's power a planar structure returns or passes, `coefficient(kx)` its
/// amplitude coefficient for the plane wave of transverse wavenumber kx (1/µm) in medium 1:
/// ∫ |A|² |c|² k_z dk_x / ∫ |A|² k_z dk_x over the propagating waves, A the Gaussian's angular
/// spectrum exp(−(k_x − k n₁ sin θ)² w²/4). Integrated in φ (k_x = k n₁ sin φ, k_z = k n₁ cos φ),
/// where the integrand is smooth; to round-off with 400 panels (800 change it by under 1e-15).
pub(crate) fn spectral_share(angle: f64, coefficient: &dyn Fn(f64) -> c64) -> f64 {
    let kn = k() * N1;
    let k0 = kn * angle.sin();
    let spectrum = |phi: f64| {
        let kx = kn * phi.sin();
        let a = (-(kx - k0).powi(2) * WAIST * WAIST / 4.0).exp();
        // |A|² k_z dk_x/dφ
        a * a * (kn * phi.cos()).powi(2)
    };
    let passed = integrate(
        &|phi| spectrum(phi) * coefficient(kn * phi.sin()).norm_sqr(),
        400,
    );
    passed / integrate(&spectrum, 400)
}

/// k_z in a medium of index n for transverse wavenumber kx: real below k n, else i|k_z|.
fn kz(n: f64, kx: f64) -> c64 {
    c64::new((k() * n).powi(2) - kx * kx, 0.0).sqrt()
}

/// Fresnel's coefficient from medium 1 to 2 for `polarization`'s U (E_y, or H_y for TM).
fn fresnel(polarization: Polarization, n1: f64, n2: f64, kx: f64) -> c64 {
    let (a, b) = match polarization {
        Polarization::Te => (kz(n1, kx), kz(n2, kx)),
        Polarization::Tm => (n2 * n2 * kz(n1, kx), n1 * n1 * kz(n2, kx)),
    };
    (a - b) / (a + b)
}

/// The film's (index N2, FILM thick, in N1) transmission of U: Airy's
/// t₁₂ t₂₁ e^(ik_z d) / (1 − r₂₁² e^(2ik_z d)), with t = 1 + r for U.
fn airy(polarization: Polarization, kx: f64) -> c64 {
    let r12 = fresnel(polarization, N1, N2, kx);
    let r21 = -r12;
    let phase = (c64::new(0.0, 1.0) * kz(N2, kx) * FILM).exp();
    (1.0 + r12) * (1.0 + r21) * phase / (1.0 - r21 * r21 * phase * phase)
}

/// The expansion's share for the beam at `angle` (radians), centred `centre` µm from the
/// window's middle, on `sections` (their first a uniform N1): (reflected, transmitted).
fn expansion_share(
    polarization: Polarization,
    angle: f64,
    centre: f64,
    sections: &[Section],
) -> Result<(f64, f64)> {
    let eme = Eme::new(polarization, Wavelength::um(LAMBDA)?, MODES)?;
    let beam = Beam::gaussian(Length::um(WAIST), Length::um(centre))?.tilted(angle, N1)?;
    let launched = eme.launch(&sections[0].slice, &beam)?;
    let s = eme.s_matrix(sections)?;
    let (r, t) = s.forward(&launched.arriving())?;
    let p = launched.power();
    Ok((power(&r, &s.first) / p, power(&t, &s.last) / p))
}

/// The largest difference of the step's reflected share from the spectral Fresnel one over
/// [`ANGLES`], centred and 2 µm off the middle, for `polarization`.
pub(crate) fn step_error(polarization: Polarization) -> Result<f64> {
    let sections = [
        Section::new(uniform(N1, WINDOW)?, Length::ZERO),
        Section::new(uniform(N2, WINDOW)?, Length::ZERO),
    ];
    let mut worst: f64 = 0.0;
    for deg in ANGLES {
        let angle = deg.to_radians();
        let want = spectral_share(angle, &|kx| fresnel(polarization, N1, N2, kx));
        for centre in [0.0, 2.0] {
            let (r, t) = expansion_share(polarization, angle, centre, &sections)?;
            worst = worst.max((r - want).abs()).max((r + t - 1.0).abs());
        }
    }
    Ok(worst)
}

/// The same through the film: its transmitted share against Airy's over the angular spectrum.
pub(crate) fn film_error(polarization: Polarization) -> Result<f64> {
    let sections = [
        Section::new(uniform(N1, WINDOW)?, Length::ZERO),
        Section::new(uniform(N2, WINDOW)?, Length::um(FILM)),
        Section::new(uniform(N1, WINDOW)?, Length::ZERO),
    ];
    let mut worst: f64 = 0.0;
    for deg in ANGLES {
        let angle = deg.to_radians();
        let want = spectral_share(angle, &|kx| airy(polarization, kx));
        let (r, t) = expansion_share(polarization, angle, 0.0, &sections)?;
        worst = worst.max((t - want).abs()).max((r + t - 1.0).abs());
    }
    Ok(worst)
}

/// The silicon slab: 220 nm of 3.473 in 1.444, in a window 30 µm wide.
const SLAB: f64 = 0.22;
const SLAB_WINDOW: f64 = 30.0;

/// A Gaussian's overlap with the slab's fundamental mode by [`Eme::launch`]'s couplings, and
/// by the exact slab's field (`mode::slab`) integrated across the whole line: the largest
/// relative difference over waists of 0.3, 1 and 2 µm, centred and 0.5 µm off, square on and at
/// 10°, for `polarization`.
pub(crate) fn slab_coupling_error(polarization: Polarization) -> Result<f64> {
    let (core, clad) = (3.473, 1.444);
    let lam = Wavelength::um(LAMBDA)?;
    let side = 0.5 * (SLAB_WINDOW - SLAB);
    let slice = Slice::new(&[
        (c64::new(clad, 0.0), Length::um(side)),
        (c64::new(core, 0.0), Length::um(SLAB)),
        (c64::new(clad, 0.0), Length::um(side)),
    ])?;
    let eme = Eme::new(polarization, lam, 4)?;
    let exact = &Slab::new(clad, core, clad, Length::um(SLAB))?.modes(polarization, lam)[0];
    let weight = |x: f64| match polarization {
        Polarization::Te => 1.0,
        Polarization::Tm => {
            let n = if (0.0..=SLAB).contains(&x) {
                core
            } else {
                clad
            };
            1.0 / (n * n)
        }
    };
    let mut worst: f64 = 0.0;
    for waist in [0.3, 1.0, 2.0] {
        for centre in [0.0, 0.5] {
            for deg in [0.0, 10.0] {
                let beam = Beam::gaussian(Length::um(waist), Length::um(centre))?
                    .tilted(f64::to_radians(deg), clad)?;
                let ours = eme.launch(&slice, &beam)?.couplings()[0];
                // x from the core's bottom; the beam's from the window's middle
                let mid = 0.5 * SLAB;
                let mut overlap = c64::new(0.0, 0.0);
                let mut mode = 0.0;
                let mut norm = 0.0;
                for (a, b) in [(-12.0, 0.0), (0.0, SLAB), (SLAB, 12.0 + SLAB)] {
                    let panels = (((b - a) / 0.02_f64).ceil() as usize).max(1);
                    let h = (b - a) / panels as f64;
                    for p in 0..panels {
                        let c = a + (p as f64 + 0.5) * h;
                        for &(node, w) in &GL20 {
                            for x in [c - 0.5 * h * node, c + 0.5 * h * node] {
                                let u = exact.field(Length::um(x)).0;
                                let f = beam.field(x - mid, k());
                                let q = 0.5 * h * w * weight(x);
                                overlap += q * f * u;
                                mode += q * u * u;
                                norm += q * f.norm_sqr();
                            }
                        }
                    }
                }
                let want = overlap.norm_sqr() / (norm * mode);
                worst = worst.max((ours - want).abs() / want);
            }
        }
    }
    Ok(worst)
}

fn zero(measured: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected: 0.0,
        tolerance,
        error: measured,
    }
}

/// The case's value, or NaN where it failed (an outcome NaN fails).
fn or_nan(r: Result<f64>) -> f64 {
    r.unwrap_or(f64::NAN)
}

/// The beam's cases, in report order.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "eme/beam-fresnel",
            title: r"A Gaussian beam (waist 3 µm, 1.55 µm) in oxide (1.444) meeting a uniform 2.0, launched on the expansion's modes (110 in a window 40 µm wide between walls: plane waves at discrete angles), tilted by 0°, 10°, 20° and 30°, centred and 2 µm off, TE and TM: its reflected share of power (largest difference shown, with $R + T - 1$)",
            tier: Tier::Analytic,
            source: r"Fresnel's $r_s$ and $r_p$ over the beam's angular spectrum, $\int \lvert A \rvert^2 \lvert r \rvert^2 k_z\thinspace dk_x / \int \lvert A \rvert^2 k_z\thinspace dk_x$ with $A = e^{-(k_x - k n_1 \sin\theta)^2 w^2/4}$, the integral in $k_x = k n_1 \sin\varphi$ to round-off",
            run: || {
                let worst =
                    or_nan(step_error(Polarization::Te)).max(or_nan(step_error(Polarization::Tm)));
                zero(super::checks::significant(worst, 2), 1e-12)
            },
        },
        Case {
            id: "eme/beam-film",
            title: r"The same beam through a film of 2.0, 0.3 µm thick, in oxide, at 0° to 30°, TE and TM: its transmitted share (largest difference shown, with $R + T - 1$)",
            tier: Tier::Analytic,
            source: r"Airy's transmission $t_{12} t_{21} e^{i k_z d}/(1 - r_{21}^2 e^{2 i k_z d})$ over the beam's angular spectrum, as eme/beam-fresnel",
            run: || {
                let worst =
                    or_nan(film_error(Polarization::Te)).max(or_nan(film_error(Polarization::Tm)));
                zero(super::checks::significant(worst, 2), 1e-12)
            },
        },
        Case {
            id: "eme/beam-slab-coupling",
            title: r"A Gaussian's overlap with a 220 nm silicon slab's fundamental mode (3.473 in 1.444, 1.55 µm, a window 30 µm wide), waists 0.3, 1 and 2 µm, centred and 0.5 µm off, at 0° and 10°, TE and TM: $\lvert\int f U w\thinspace dx\rvert^2 / (\int \lvert f\rvert^2 w\thinspace dx \int U^2 w\thinspace dx)$ from the launched amplitudes (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"the exact slab's mode (`mode::slab`, Chrostowski and Hochberg's Listing 3.3) and the same Gaussian integrated across the line to round-off; $w = 1$ for TE, $1/n^2$ for TM",
            run: || {
                let worst = or_nan(slab_coupling_error(Polarization::Te))
                    .max(or_nan(slab_coupling_error(Polarization::Tm)));
                zero(super::checks::significant(worst, 2), 1e-12)
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_beams_shares_and_overlaps_are_the_closed_forms_to_round_off() {
        for pol in [Polarization::Te, Polarization::Tm] {
            assert!(step_error(pol).unwrap() < 1e-12);
            assert!(film_error(pol).unwrap() < 1e-12);
            assert!(slab_coupling_error(pol).unwrap() < 1e-12);
        }
    }
}
