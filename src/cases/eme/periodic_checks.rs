//! Periodic sections ([`super::Bloch`], [`super::Eme::grating`]): a quarter-wave stack's Bloch
//! modes against the two layers' closed form, its band edges against the quarter-wave stop
//! band's width, and a slab grating's N periods by repeated squaring against the same sections
//! in sequence.

use num_complex::Complex64 as c64;

use super::checks::half;
use super::{Eme, Section, Slice, Wall};
use crate::mode::Polarization;
use crate::units::{Length, Wavelength};
use crate::validation::{Case, Outcome, Tier};

/// The stack's low index: silica.
pub(crate) const N1: f64 = 1.444;
/// The stack's high index: silicon nitride.
pub(crate) const N2: f64 = 2.0;
/// The wavelength its layers are a quarter wave thick at, µm.
pub(crate) const CENTRE: f64 = 1.55;

/// A layer's thickness, a quarter wave at [`CENTRE`] in index `n`, µm.
fn quarter(n: f64) -> f64 {
    CENTRE / (4.0 * n)
}

/// The two layers' closed form, cos KΛ = cos k₁d₁ cos k₂d₂ − ½(β₁/β₂ + β₂/β₁) sin k₁d₁ sin k₂d₂
/// (the half-trace of their transfer matrices), with each layer's propagation constant
/// βᵢ = k₀ nᵢ (and kᵢdᵢ = βᵢdᵢ): TE's admittances are the propagation constants.
pub(crate) fn closed_form(b1: c64, d1: f64, b2: c64, d2: f64) -> c64 {
    let (p1, p2) = (b1 * d1, b2 * d2);
    p1.cos() * p2.cos() - 0.5 * (b1 / b2 + b2 / b1) * p1.sin() * p2.sin()
}

/// The quarter-wave stack's period as sections across a window `width` µm wide: each layer
/// uniform across it, so that between magnetic walls TE's m-th mode is cos(mπx/W) in both, with
/// n_eff² = n² − (mλ/2W)², and each mode meets only its own at the interfaces.
pub(crate) fn stack_period(width: f64) -> Vec<Section> {
    let uniform =
        |n: f64| Slice::new(&[(c64::new(n, 0.0), Length::um(width))]).expect("a uniform slice");
    vec![
        Section::new(uniform(N1), Length::um(quarter(N1))),
        Section::new(uniform(N2), Length::um(quarter(N2))),
    ]
}

/// The expansion's settings for the stack: TE between magnetic walls, `modes` modes.
pub(crate) fn stack_eme(wavelength: f64, modes: usize) -> crate::Result<Eme> {
    Ok(
        Eme::new(Polarization::Te, Wavelength::um(wavelength)?, modes)?
            .with_walls(Wall::Magnetic, Wall::Magnetic),
    )
}

/// The stack's Bloch modes' cos KΛ against the closed form at `wavelengths` (µm), in a window
/// `width` µm wide with `modes` modes: the largest departure, |Δ cos KΛ| / max(1, |cos KΛ|),
/// each mode's closed form matched to the nearest of the expansion's.
pub(crate) fn stack_departure(wavelengths: &[f64], width: f64, modes: usize) -> f64 {
    let period = stack_period(width);
    let mut worst = 0.0f64;
    for &l in wavelengths {
        let bloch = stack_eme(l, modes)
            .and_then(|e| e.bloch(&period))
            .expect("the stack's Bloch modes");
        let k0 = std::f64::consts::TAU / l;
        for m in 0..modes {
            let beta = |n: f64| {
                let q = m as f64 * l / (2.0 * width);
                c64::new(n * n - q * q, 0.0).sqrt() * k0
            };
            let c = closed_form(beta(N1), quarter(N1), beta(N2), quarter(N2));
            let d = bloch
                .cos
                .iter()
                .map(|b| (b - c).norm() / c.norm().max(1.0))
                .fold(f64::INFINITY, f64::min);
            worst = worst.max(d);
        }
    }
    worst
}

/// The wavelengths of the closed-form check: 61 from 1 to 2.5 µm, the first stop band's
/// (1.41 to 1.73 µm) among them.
pub(crate) fn stack_wavelengths() -> Vec<f64> {
    (0..61).map(|k| 1.0 + 0.025 * k as f64).collect()
}

/// Re cos KΛ of the stack's one mode at ν = ω/ω₀ (the wavelength [`CENTRE`]/ν).
fn stack_cos(nu: f64) -> f64 {
    stack_eme(CENTRE / nu, 1)
        .and_then(|e| e.bloch(&stack_period(1.0)))
        .expect("the stack's Bloch mode")
        .cos[0]
        .re
}

/// The edge of the first stop band between ν = `inside` (cos KΛ < −1) and `outside` (> −1), by
/// bisection until the two ends are neighbouring floats.
fn edge(mut inside: f64, mut outside: f64) -> f64 {
    loop {
        let mid = 0.5 * (inside + outside);
        if mid == inside || mid == outside {
            return mid;
        }
        if stack_cos(mid) < -1.0 {
            inside = mid;
        } else {
            outside = mid;
        }
    }
}

/// The first stop band's edges, ν₋ and ν₊, by bisection on cos KΛ = −1 from its centre (ν = 1).
pub(crate) fn band_edges() -> (f64, f64) {
    (edge(1.0, 0.5), edge(1.0, 1.5))
}

/// The quarter-wave stop band's width, Δω/ω₀ = (4/π) arcsin((n₂ − n₁)/(n₂ + n₁)).
pub(crate) fn band_width() -> f64 {
    4.0 / std::f64::consts::PI * ((N2 - N1) / (N2 + N1)).asin()
}

/// The slab grating's settings at `wavelength` µm: half of it (TE's even modes, a magnetic
/// wall at its plane of symmetry), 1.5 µm to a PML 4 µm thick of strength 0.1, 20 modes.
pub(crate) fn grating_eme(wavelength: f64) -> crate::Result<Eme> {
    Eme::new(Polarization::Te, Wavelength::um(wavelength)?, 20)?
        .with_walls(Wall::Magnetic, Wall::Electric)
        .with_upper_pml(Length::um(4.0), 0.1)
}

/// The slab grating's period: 300 nm of silicon slab (3.473 in silica, 1.444), then 220 nm,
/// each 0.14 µm long.
pub(crate) fn grating_period() -> Vec<Section> {
    vec![
        Section::new(half(0.30, 1.5), Length::um(0.14)),
        Section::new(half(0.22, 1.5), Length::um(0.14)),
    ]
}

/// The slab grating of `count` periods between two 220 nm slabs at `wavelength` µm: by
/// repeated squaring and in sequence.
pub(crate) fn grating_pair(
    wavelength: f64,
    count: usize,
) -> (super::Scattering, super::Scattering) {
    let eme = grating_eme(wavelength).expect("the grating's settings");
    let port = Section::new(half(0.22, 1.5), Length::ZERO);
    let mut sections = vec![port.clone()];
    sections.extend(grating_period());
    sections.push(port.clone());
    let squared = eme.grating(&sections, 1..3, count).expect("by squaring");
    let mut sequence = vec![port.clone()];
    for _ in 0..count {
        sequence.extend(grating_period());
    }
    sequence.push(port);
    (squared, eme.s_matrix(&sequence).expect("in sequence"))
}

/// The largest |a_qp − b_qp| over the four blocks.
pub(crate) fn departure(a: &super::Scattering, b: &super::Scattering) -> f64 {
    [
        (&a.s11, &b.s11),
        (&a.s12, &b.s12),
        (&a.s21, &b.s21),
        (&a.s22, &b.s22),
    ]
    .iter()
    .flat_map(|(x, y)| x.iter().zip(y.iter()))
    .flat_map(|(r, q)| r.iter().zip(q.iter()).map(|(u, v)| (u - v).norm()))
    .fold(0.0, f64::max)
}

/// The grating's periods in the squaring case: 100, 1100100 in binary, so that the squaring
/// both squares and multiplies.
pub(crate) const PERIODS: usize = 100;

fn zero(measured: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected: 0.0,
        tolerance,
        error: measured,
    }
}

/// Periodic sections' cases, in report order.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "eme/bloch-quarter-wave",
            title: r"Bloch modes from one period's S-matrix: a quarter-wave stack at 1.55 µm (silica, 1.444, and silicon nitride, 2.0, each layer uniform across a window between magnetic walls, TE's one uniform mode), $\cos K\Lambda$ at 61 wavelengths from 1 to 2.5 µm, the first stop band (1.41 to 1.73 µm) among them: the largest departure from the closed form, $\lvert \Delta \cos K\Lambda \rvert / \max(1, \lvert \cos K\Lambda \rvert)$ (shown)",
            tier: Tier::Analytic,
            source: r"the two layers' transfer matrices' half-trace, $\cos K\Lambda = \cos k_1 d_1 \cos k_2 d_2 - \tfrac12 (n_1/n_2 + n_2/n_1) \sin k_1 d_1 \sin k_2 d_2$ (P. Yeh, A. Yariv, C.-S. Hong, J. Opt. Soc. Am. 67, 423 (1977), doi:10.1364/JOSA.67.000423, Eq. (27)); the uniform mode's $n_\text{eff}$ is the layer's index and the interface's S is Fresnel's exactly (`eme/step-fresnel-te`): round-off",
            run: || zero(stack_departure(&stack_wavelengths(), 1.0, 1), 1e-12),
        },
        Case {
            id: "eme/bloch-quarter-wave-modes",
            title: r"The same stack in a window 2 µm wide with 8 modes, $\cos(m\pi x/W)$ in each layer with $n_\text{eff}^2 = n^2 - (m\lambda/2W)^2$, propagating and evanescent, at 1.3, 1.55 and 1.8 µm: each mode's closed form against the nearest of the 8 Bloch modes, the largest departure as above (shown)",
            tier: Tier::Analytic,
            source: r"each mode meets only its own at the interfaces, so each is a two-layer stack of its own: the closed form above with $\beta_i = k_0 n_{\text{eff},i}$ in place of $k_0 n_i$ (TE's admittances); the eigenproblem of all 16 waves at once, which never inverts $S_{12}$: round-off",
            run: || zero(stack_departure(&[1.3, 1.55, 1.8], 2.0, 8), 1e-12),
        },
        Case {
            id: "eme/bloch-band-edges",
            title: r"The same stack's first stop band: its edges by bisection on $\cos K\Lambda = -1$ from the expansion, to neighbouring floats in $\omega/\omega_0$, their distance $\Delta\omega/\omega_0$ (shown) against the quarter-wave stop band's width",
            tier: Tier::Analytic,
            source: r"$\Delta\omega/\omega_0 = (4/\pi) \arcsin((n_2 - n_1)/(n_2 + n_1))$, the closed form at quarter-wave thicknesses, where it reaches $-1$ at $\cos(\tfrac{\pi}{2}\omega/\omega_0) = \pm(n_2 - n_1)/(n_2 + n_1)$; the slope of $\cos K\Lambda$ there is about 1, so the edges are at round-off",
            run: || {
                let (lo, hi) = band_edges();
                let expected = band_width();
                Outcome {
                    measured: hi - lo,
                    expected,
                    tolerance: 1e-12 * expected,
                    error: (hi - lo - expected).abs(),
                }
            },
        },
        Case {
            id: "eme/grating-squaring",
            title: r"100 periods by repeated squaring: a silicon slab grating (3.473 in 1.444; 300 and 220 nm thick, 0.14 µm each; TE, half of it, 20 even modes, a PML 4 µm thick) between two 220 nm slabs at 1.55 µm, its S-matrix from the period's S taken to the 99th power by repeated squaring and joined to the last period and the ports (11 star products) against the 200 sections in sequence (199 interfaces): the largest $\lvert \Delta S_{qp} \rvert$ over the four blocks (shown)",
            tier: Tier::Analytic,
            source: r"the star product is associative, so the two orders of the same products agree but for round-off",
            run: || {
                let (squared, sequence) = grating_pair(1.55, PERIODS);
                zero(departure(&squared, &sequence), 1e-12)
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_uniform_section_is_its_own_bloch_mode() {
        // one section 0.4 um long: its modes are its Bloch modes, cos KΛ = cos βΛ
        let slice = half(0.22, 1.5);
        let eme = Eme::new(Polarization::Te, Wavelength::um(1.55).expect("1.55"), 6)
            .expect("an eme")
            .with_walls(Wall::Magnetic, Wall::Electric);
        let modes = eme.modes(&slice).expect("modes");
        let bloch = eme
            .bloch(&[Section::new(slice, Length::um(0.4))])
            .expect("bloch");
        let k0 = std::f64::consts::TAU / 1.55;
        for m in &modes {
            let c = (m.effective_index() * k0 * 0.4).cos();
            let near = bloch
                .cos
                .iter()
                .map(|b| (b - c).norm() / c.norm().max(1.0))
                .fold(f64::INFINITY, f64::min);
            assert!(near < 1e-12, "{c} not among {:?}", bloch.cos);
        }
    }

    #[test]
    #[ignore = "prints the numbers"]
    fn the_periodic_study() {
        eprintln!(
            "one mode: {:e}",
            stack_departure(&stack_wavelengths(), 1.0, 1)
        );
        eprintln!(
            "eight modes: {:e}",
            stack_departure(&[1.3, 1.55, 1.8], 2.0, 8)
        );
        for (count, l) in [(25usize, 1.55), (25, 1.45), (100, 1.55)] {
            let t = std::time::Instant::now();
            let (sq, seq) = grating_pair(l, count);
            let d = departure(&sq, &seq);
            eprintln!(
                "grating {count} at {l}: {d:e}, |S21|^2 {:.6}, {:.1} s",
                sq.s21[0][0].norm_sqr(),
                t.elapsed().as_secs_f64()
            );
        }
        for modes in [10, 20, 40, 80, 160] {
            for (thick, strength) in [(4.0, 0.1), (8.0, 0.05)] {
                let e = Eme::new(Polarization::Te, Wavelength::um(1.55).expect("l"), modes)
                    .expect("e")
                    .with_walls(Wall::Magnetic, Wall::Electric)
                    .with_upper_pml(Length::um(thick), strength)
                    .expect("pml");
                match e.bloch(&grating_period()) {
                    Ok(b) => eprintln!(
                        "{modes} modes, PML {thick} um of {strength}: fundamental {:.8} {:.3e}",
                        b.effective_indices[0].re, b.effective_indices[0].im
                    ),
                    Err(err) => eprintln!("{modes} modes, PML {thick}/{strength}: {err}"),
                }
            }
        }
        for l in [1.45, 1.5, 1.55, 1.6, 1.65] {
            let e = grating_eme(l).expect("e");
            let b = e.bloch(&grating_period()).expect("b");
            eprintln!(
                "bloch at {l}: cos {:.4} n {:.5} | {:?}",
                b.cos[0],
                b.effective_indices[0],
                &b.effective_indices[1..4]
            );
        }
        let (lo, hi) = band_edges();
        eprintln!(
            "edges {lo} {hi}: width {} against {}, {:e}",
            hi - lo,
            band_width(),
            ((hi - lo) / band_width() - 1.0).abs()
        );
    }
}
