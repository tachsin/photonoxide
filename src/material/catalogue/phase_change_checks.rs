//! The validation cases of the phase-change materials: the fits against Delaney et al.'s data,
//! the paper's contrast and loss at 1550 nm, the crystallization its anneals show, and both
//! states through the slab's closed form.

use num_complex::Complex64 as c64;

use super::phase_change::{self, DELANEY_SB2S3_ANNEALS, DELANEY_SB2SE3_ANNEALS, crystallized_at};
use crate::Result;
use crate::material::Material;
use crate::mode::Polarization;
use crate::mode::slab::Slab;
use crate::mode::slab_fd::Profile;
use crate::units::{Length, Wavelength};
use crate::validation::Outcome;

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

fn index(m: &Result<Material>, um: f64) -> c64 {
    m.as_ref()
        .ok()
        .and_then(|m| m.refractive_index(Wavelength::um(um).ok()?).ok())
        .unwrap_or(c64::new(f64::NAN, f64::NAN))
}

/// Delaney's data (the data set's `3_ellipsometry`) at four wavelengths (µm, n, k) per state:
/// the points nearest 1.0, 1.31, 1.55 and 1.69 µm.
type Points = [(f64, f64, f64); 4];

pub(crate) const SB2SE3_A_POINTS: Points = [
    (1.012235, 3.395580, 0.0),
    (1.310051, 3.304707, 0.0),
    (1.551182, 3.285328, 0.0),
    (1.689679, 3.283947, 0.0),
];
pub(crate) const SB2SE3_C_POINTS: Points = [
    (1.200289, 4.181685, 0.002427),
    (1.310051, 4.121314, 0.000981),
    (1.551182, 4.049704, 0.000213),
    (1.689679, 4.029383, 0.000109),
];
pub(crate) const SB2S3_A_POINTS: Points = [
    (1.012235, 2.844049, 0.0),
    (1.310051, 2.767051, 0.0),
    (1.551182, 2.718479, 0.0),
    (1.689679, 2.691693, 0.0),
];
pub(crate) const SB2S3_C_POINTS: Points = [
    (1.012235, 3.434409, 0.0),
    (1.310051, 3.343246, 0.0),
    (1.551182, 3.308204, 0.0),
    (1.689679, 3.294580, 0.0),
];
pub(crate) const GST_A_POINTS: Points = [
    (1.012235, 4.680758, 0.745742),
    (1.310051, 4.543957, 0.258868),
    (1.551182, 4.352406, 0.060228),
    (1.689679, 4.242269, 0.010407),
];
pub(crate) const GST_C_POINTS: Points = [
    (1.012235, 7.547787, 2.890401),
    (1.310051, 7.967662, 1.682726),
    (1.551182, 7.912606, 1.030550),
    (1.689679, 7.829783, 0.772746),
];

/// The largest deviation of a material from its points, in n or k.
fn worst(m: Result<Material>, points: &Points) -> f64 {
    points
        .iter()
        .map(|&(um, n, k)| {
            let i = index(&m, um);
            (i.re - n).abs().max((i.im - k).abs())
        })
        .fold(0.0, f64::max)
}

/// The four Sellmeier fits (with crystalline Sb₂Se₃'s k) against the data: the largest
/// deviation.
pub(crate) fn transparent_fits() -> Outcome {
    let w = [
        worst(phase_change::sb2se3_amorphous(), &SB2SE3_A_POINTS),
        worst(phase_change::sb2se3_crystalline(), &SB2SE3_C_POINTS),
        worst(phase_change::sb2s3_amorphous(), &SB2S3_A_POINTS),
        worst(phase_change::sb2s3_crystalline(), &SB2S3_C_POINTS),
    ]
    .into_iter()
    .fold(0.0, f64::max);
    outcome(w, 0.0, 1e-4)
}

/// GST's two Lorentz fits against the data: the largest deviation in n or k.
pub(crate) fn gst_fits() -> Outcome {
    let w = worst(phase_change::gst_amorphous(), &GST_A_POINTS)
        .max(worst(phase_change::gst_crystalline(), &GST_C_POINTS));
    outcome(w, 0.0, 3e-3)
}

/// The contrast at 1550 nm, crystalline less amorphous, of (Sb₂Se₃, Sb₂S₃).
pub(crate) fn contrasts() -> (f64, f64) {
    let dn = |c: Result<Material>, a: Result<Material>| index(&c, 1.55).re - index(&a, 1.55).re;
    (
        dn(
            phase_change::sb2se3_crystalline(),
            phase_change::sb2se3_amorphous(),
        ),
        dn(
            phase_change::sb2s3_crystalline(),
            phase_change::sb2s3_amorphous(),
        ),
    )
}

/// The contrasts at 1550 nm against the abstract's Δn = 0.77 (Sb₂Se₃) and 0.60 (Sb₂S₃): the
/// larger deviation.
pub(crate) fn contrast() -> Outcome {
    let (se, s) = contrasts();
    let error = (se - 0.77).abs().max((s - 0.60).abs());
    Outcome {
        measured: error,
        expected: 0.0,
        tolerance: 0.015,
        error,
    }
}

/// The k at 1550 nm of amorphous Sb₂Se₃ and both states of Sb₂S₃, against the abstract's k <
/// 1e-5: the largest.
pub(crate) fn lossless() -> Outcome {
    let k = [
        phase_change::sb2se3_amorphous(),
        phase_change::sb2s3_amorphous(),
        phase_change::sb2s3_crystalline(),
    ]
    .iter()
    .map(|m| index(m, 1.55).im.abs())
    .fold(0.0, f64::max);
    outcome(k, 0.0, 1e-5)
}

/// Sb₂Se₃'s crystallization in the anneals (the first after which its index stands past half
/// its jump), K, against the 473 K Lawson et al.'s model takes; within the anneals' 25 K step.
pub(crate) fn sb2se3_crystallization() -> Outcome {
    outcome(crystallized_at(&DELANEY_SB2SE3_ANNEALS), 473.0, 25.0)
}

/// Sb₂S₃'s, against the catalogue's (the same rule's), within the anneals' 10 K step: the
/// catalogue keeps what the data shows.
pub(crate) fn sb2s3_crystallization() -> Outcome {
    let catalogue = phase_change::phase_change("sb2s3")
        .and_then(|p| p.crystallization.map(|c| c.value))
        .unwrap_or(f64::NAN);
    outcome(crystallized_at(&DELANEY_SB2S3_ANNEALS), catalogue, 10.0)
}

/// A slab of Sb₂Se₃ `thickness_nm` thick on silica (1.444) under air at 1550 nm, TE₀, in a
/// state: its effective index by the closed form and by finite differences at `step_nm`.
fn slab(crystalline: bool, thickness_nm: f64, step_nm: f64) -> (f64, f64) {
    let m = if crystalline {
        phase_change::sb2se3_crystalline()
    } else {
        phase_change::sb2se3_amorphous()
    };
    // the closed form takes a real index: crystalline Sb₂Se₃'s k of 2e-4 is left out of both
    let n = index(&m, 1.55).re;
    let Ok(lam) = Wavelength::um(1.55) else {
        return (f64::NAN, f64::NAN);
    };
    let exact = Slab::new(1.444, n, 1.0, Length::nm(thickness_nm))
        .ok()
        .and_then(|s| s.modes(Polarization::Te, lam).first().copied())
        .map_or(f64::NAN, |m| m.effective_index());
    // 2 µm of silica below, 2 µm of air above, the film's faces on nodes
    let d = thickness_nm * 1e-3;
    let cells = ((4.0 + d) / (step_nm * 1e-3)).round() as usize;
    let fd = Profile::uniform(-2.0, 2.0 + d, cells, |x| {
        let e = if x < 0.0 {
            1.444 * 1.444
        } else if x < d {
            n * n
        } else {
            1.0
        };
        c64::new(e, 0.0)
    })
    .and_then(|p| p.modes(Polarization::Te, lam, 1, None))
    .ok()
    .and_then(|m| m.first().map(|m| m.effective_index.re))
    .unwrap_or(f64::NAN);
    (exact, fd)
}

/// Both states of a 300 nm Sb₂Se₃ slab, TE₀ at 1550 nm, by finite differences at 2.5 nm against
/// the closed form: the larger error.
pub(crate) fn slab_closed_form() -> Outcome {
    let error = [false, true]
        .into_iter()
        .map(|c| {
            let (exact, fd) = slab(c, 300.0, 2.5);
            (fd - exact).abs()
        })
        .fold(0.0, f64::max);
    outcome(error, 0.0, 1e-4)
}

/// The same slab's order of convergence from 10 to 5 nm (crystalline, the higher contrast).
pub(crate) fn slab_order() -> Outcome {
    let (exact, coarse) = slab(true, 300.0, 10.0);
    let (_, fine) = slab(true, 300.0, 5.0);
    let order = ((coarse - exact).abs() / (fine - exact).abs()).log2();
    outcome(order, 2.0, 0.2)
}

/// Delaney et al.'s Table 1 at 1060, 1310 and 1550 nm: (state, [n at each]), crystalline
/// Sb₂Se₃ from 1310 nm (its fit starts at 1.2 µm). k is 0 at these for all but crystalline
/// Sb₂Se₃ (0.010 and 0.001 at 1060 and 1310 nm).
type Row = (&'static str, &'static [(f64, f64)]);
pub(crate) const TABLE_1: [Row; 4] = [
    (
        "sb2s3-amorphous",
        &[(1.06, 2.829), (1.31, 2.767), (1.55, 2.712)],
    ),
    (
        "sb2s3-crystalline",
        &[(1.06, 3.413), (1.31, 3.343), (1.55, 3.308)],
    ),
    (
        "sb2se3-amorphous",
        &[(1.06, 3.372), (1.31, 3.305), (1.55, 3.285)],
    ),
    ("sb2se3-crystalline", &[(1.31, 4.121), (1.55, 4.050)]),
];

fn state_material(id: &str) -> Result<Material> {
    match id {
        "sb2s3-amorphous" => phase_change::sb2s3_amorphous(),
        "sb2s3-crystalline" => phase_change::sb2s3_crystalline(),
        "sb2se3-amorphous" => phase_change::sb2se3_amorphous(),
        _ => phase_change::sb2se3_crystalline(),
    }
}

/// The deviations of the fits from Table 1's printed n, each with its state and wavelength.
pub(crate) fn table_1_deviations() -> Vec<(&'static str, f64, f64)> {
    TABLE_1
        .iter()
        .flat_map(|(id, rows)| {
            let m = state_material(id);
            rows.iter()
                .map(move |&(um, n)| (*id, um, index(&m, um).re - n))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The fits against Table 1, its three decimals: the largest deviation but amorphous Sb₂S₃'s
/// at 1550 nm, which the table prints 0.0065 below the data set's own.
pub(crate) fn table_1() -> Outcome {
    let worst = table_1_deviations()
        .into_iter()
        .filter(|&(id, um, _)| !(id == "sb2s3-amorphous" && um == 1.55))
        .map(|(_, _, d)| d.abs())
        .fold(0.0, f64::max);
    outcome(worst, 0.0, 5e-4)
}

/// Delaney et al.'s insertion loss (dB) against the length of the PCM on the waveguide (0 to
/// 800 µm in steps of 100), crystalline Sb₂Se₃ and Sb₂S₃: the data set's 6_SWG_losses (Fig. 7a,
/// b), the means over their devices.
pub(crate) const LOSS_SB2SE3_C: [f64; 9] = [
    10.1274724,
    12.36525157,
    11.65707187,
    14.69465876,
    13.843549,
    14.98606453,
    14.97194983,
    16.76093294,
    19.50388553,
];
pub(crate) const LOSS_SB2S3_C: [f64; 9] = [
    13.40484233,
    13.81751537,
    15.13217838,
    27.14754392,
    26.64229666,
    29.93527523,
    34.61874844,
    29.06067737,
    34.72640488,
];

/// The slope (dB/µm) of the least-squares line through losses at 0, 100, … 800 µm.
pub(crate) fn slope(losses: &[f64; 9]) -> f64 {
    let x: Vec<f64> = (0..9).map(|i| 100.0 * i as f64).collect();
    let (mx, my) = (
        x.iter().sum::<f64>() / 9.0,
        losses.iter().sum::<f64>() / 9.0,
    );
    let sxy: f64 = x.iter().zip(losses).map(|(a, b)| (a - mx) * (b - my)).sum();
    let sxx: f64 = x.iter().map(|a| (a - mx) * (a - mx)).sum();
    sxy / sxx
}

/// The crystalline films' propagation loss, the slopes of lines through the data set's
/// losses, against the paper's 0.01 (Sb₂Se₃) and 0.028 dB/µm (Sb₂S₃): the larger relative
/// deviation.
pub(crate) fn losses() -> Outcome {
    let d = (slope(&LOSS_SB2SE3_C) / 0.01 - 1.0)
        .abs()
        .max((slope(&LOSS_SB2S3_C) / 0.028 - 1.0).abs());
    outcome(d, 0.0, 0.05)
}

/// Delaney et al.'s figure of merit, Eq. (1), from their Δn_eff (0.0717 for Sb₂Se₃, 0.0356 for
/// Sb₂S₃, a 25 nm film on a 500 nm silicon rib) and the slopes of [`losses`], against their
/// 29.0 and 5.15 rad/dB: the larger relative deviation.
pub(crate) fn figures_of_merit() -> Outcome {
    let Ok(lam) = Wavelength::um(1.55) else {
        return outcome(f64::NAN, 0.0, 0.05);
    };
    let fom = |dn, loss: &[f64; 9]| phase_change::figure_of_merit(dn, slope(loss), lam);
    let d = (fom(0.0717, &LOSS_SB2SE3_C) / 29.0 - 1.0)
        .abs()
        .max((fom(0.0356, &LOSS_SB2S3_C) / 5.15 - 1.0).abs());
    outcome(d, 0.0, 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_case_passes() {
        for (name, o) in [
            ("transparent", transparent_fits()),
            ("gst", gst_fits()),
            ("contrast", contrast()),
            ("lossless", lossless()),
            ("sb2se3 tc", sb2se3_crystallization()),
            ("sb2s3 tc", sb2s3_crystallization()),
            ("slab", slab_closed_form()),
            ("order", slab_order()),
            ("table 1", table_1()),
            ("losses", losses()),
            ("fom", figures_of_merit()),
        ] {
            assert!(o.error <= o.tolerance, "{name}: {o:?}");
        }
        let (se, s) = contrasts();
        assert!(
            (se - 0.7644).abs() < 1e-3 && (s - 0.5897).abs() < 1e-3,
            "{se} {s}"
        );
        // the one Table 1 value the data set doesn't give
        let odd = table_1_deviations()
            .into_iter()
            .find(|&(id, um, _)| id == "sb2s3-amorphous" && um == 1.55)
            .unwrap();
        assert!((odd.2 - 0.0065).abs() < 5e-4, "{odd:?}");
        assert!((slope(&LOSS_SB2SE3_C) - 0.0096).abs() < 1e-4);
        assert!((slope(&LOSS_SB2S3_C) - 0.0288).abs() < 1e-4);
    }

    #[test]
    fn a_partly_crystalline_film_lies_between_its_states() {
        let (a, c) = (
            phase_change::gst_amorphous().unwrap(),
            phase_change::gst_crystalline().unwrap(),
        );
        let lam = Wavelength::um(1.55).unwrap();
        let at = |f| phase_change::partly_crystalline(&a, &c, f, lam).unwrap();
        assert!((at(0.0) - a.refractive_index(lam).unwrap()).norm() < 1e-12);
        assert!((at(1.0) - c.refractive_index(lam).unwrap()).norm() < 1e-12);
        let half = at(0.5);
        assert!(half.re > at(0.0).re && half.re < at(1.0).re);
        assert!(phase_change::partly_crystalline(&a, &c, 1.5, lam).is_err());
    }

    #[test]
    fn the_catalogue_keeps_the_anneals_and_its_sources() {
        let se = phase_change::phase_change("sb2se3").unwrap();
        assert_eq!(
            se.crystallization.as_ref().unwrap().value,
            crystallized_at(&DELANEY_SB2SE3_ANNEALS)
        );
        // ρ c_p from Efthimiopoulos' volume and Lawson's c_p
        assert!((se.density.as_ref().unwrap().value - 5849.0).abs() < 1.0);
        assert!((se.heat_capacity().unwrap() - 2.9654e6).abs() < 1e3);
        for p in phase_change::phase_changes() {
            let keys: Vec<&str> = p.references.iter().map(|r| r.key.as_str()).collect();
            for c in [
                &p.crystallization,
                &p.melting,
                &p.amorphous_conductivity,
                &p.crystalline_conductivity,
                &p.specific_heat,
                &p.density,
            ]
            .into_iter()
            .flatten()
            {
                assert!(keys.contains(&c.source.reference.as_str()), "{}", p.id);
            }
            for m in &p.missing {
                assert!(m.reason.ends_with("coming"), "{}", m.property);
            }
            for state in [
                super::super::State::Amorphous,
                super::super::State::Crystalline,
            ] {
                let e = super::super::entry(p.entry(state)).unwrap();
                assert_eq!(e.category, super::super::Category::PhaseChange);
            }
        }
        assert!(phase_change::phase_change("gst").unwrap().melting.is_none());
        let gst = phase_change::phase_change("gst").unwrap();
        assert_eq!(
            gst.conductivity(super::super::State::Crystalline)
                .unwrap()
                .value,
            0.57
        );
        assert_eq!(
            gst.state_of("gst-amorphous"),
            Some(super::super::State::Amorphous)
        );
    }

    #[test]
    fn frantz_and_delaney_disagree_on_crystalline_gst() {
        let d = index(&phase_change::gst_crystalline(), 1.55);
        let f = index(&phase_change::frantz_crystalline(), 1.55);
        assert!(
            (d.re - 7.91).abs() < 0.01 && (f.re - 6.46).abs() < 0.01,
            "{d} {f}"
        );
        let a = index(&phase_change::frantz_amorphous(), 1.31);
        assert!(
            (a.re - 4.272).abs() < 2e-3 && (a.im - 0.147).abs() < 2e-3,
            "{a}"
        );
    }
}
