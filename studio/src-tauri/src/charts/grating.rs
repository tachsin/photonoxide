//! The grating coupler's charts: a shallow-etched grating in a 220 nm silicon film on a buried
//! oxide, TE, coupling to a fibre's Gaussian mode at an angle θ from the normal.
//!
//! - **Phase matching** (`grating-angle`, `grating-band`): the grating's effective index is the
//!   fill-weighted mean of two slabs' exact TE₀ indices, the unetched film's and the etched one's
//!   (`mode::slab`), a first-order estimate; the first diffraction order leaves at
//!   n_c sin θ = n_eff − λ/Λ. A Gaussian beam of waist w₀ holds a spread of wavevectors along the
//!   surface, so the coupling falls as exp(−q²w²/4) with the mismatch q and the beam's footprint
//!   w = w₀/cos θ: the bandwidth Passoni et al. 2017 (doi:10.1063/1.4974992) call the finite
//!   size's, Gaussian in shape. The indices are held fixed over wavelength (only the guide's own
//!   dispersion), and the group index is a central difference over ±1 nm.
//! - **The overlap** (`grating-overlap`, `grating-apodization`): the field a grating sends up,
//!   √(2α(z)P(z)) with dP/dz = −2αP, against the fibre's Gaussian on the surface, summed on steps
//!   of w/400; the uniform grating's best is Tamir and Peng 1977's 80 % (doi:10.1007/BF00882729,
//!   their Eq. 6a); the apodized grating follows Taillaert et al. 2006's Eq. 2.1
//!   (doi:10.1143/JJAP.45.6071) until its scattering reaches the strongest the grating has.
//! - **The directionality** (`grating-directionality`): a sheet at the grooves' mid-depth
//!   radiating equally up and down, between the reflections of the stack above and below it, both
//!   by the transfer matrices of `mode::multilayer` (Chilwell and Hodgkinson 1984): the upward share
//!   |1 + r_b|²(1 − |r_t|²) over the sum with |1 + r_t|²(1 − |r_b|²).

use std::f64::consts::PI;

use num_complex::Complex64 as c64;
use photonoxide::mode::Polarization;
use photonoxide::mode::multilayer::Multilayer;
use photonoxide::mode::slab::Slab;
use photonoxide::units::{Length, Wavelength};

use super::slab::{OXIDE, SILICON};
use super::{ChartData, ChartSpec, Curve, Figure, figure, invalid, param, sig};

/// The silicon film, µm: the standard SOI wafer's 220 nm.
pub const FILM: f64 = 0.22;
/// The wavelength the charts are designed at, µm.
const CENTRE: f64 = 1.55;
/// The palette's grey, for reference lines.
const GREY: usize = 7;
/// Steps per footprint half-width w in the overlap sums.
const STEPS: f64 = 400.0;

/// A shallow-etched grating in the film: its etch, µm, the teeth's (unetched) share of the
/// period, and the cladding's index over it.
#[derive(Clone, Copy, Debug)]
pub struct Grating {
    pub etch: f64,
    pub fill: f64,
    pub cladding: f64,
}

impl Grating {
    /// TE₀'s effective index of a silicon film `t` µm thick on oxide under the cladding, at `l` µm.
    fn film_index(&self, t: f64, l: f64) -> photonoxide::Result<f64> {
        Slab::new(OXIDE, SILICON, self.cladding, Length::um(t))?
            .modes(Polarization::Te, Wavelength::um(l)?)
            .first()
            .map(|m| m.effective_index())
            .ok_or_else(|| {
                invalid(
                    "grating",
                    format!("a film {} nm thick guides no TE mode", sig(t * 1e3, 4)),
                )
            })
    }

    /// The unetched film's and the etched film's TE₀ indices at `l` µm.
    pub fn indices(&self, l: f64) -> photonoxide::Result<(f64, f64)> {
        Ok((
            self.film_index(FILM, l)?,
            self.film_index(FILM - self.etch, l)?,
        ))
    }

    /// The grating's effective index at `l` µm: the two films' weighted by their shares.
    pub fn index(&self, l: f64) -> photonoxide::Result<f64> {
        let (tooth, groove) = self.indices(l)?;
        Ok(self.fill * tooth + (1.0 - self.fill) * groove)
    }

    /// The grating's group index n − λ dn/dλ at `l` µm, by a central difference over ±1 nm.
    pub fn group_index(&self, l: f64) -> photonoxide::Result<f64> {
        const H: f64 = 1e-3;
        let slope = (self.index(l + H)? - self.index(l - H)?) / (2.0 * H);
        Ok(self.index(l)? - l * slope)
    }
}

/// How far the band moves per degree of the fibre's tilt, nm: dλ/dθ = λ n_c cos θ/(n_g − n_c sin θ).
pub fn nm_per_degree(l: f64, n_g: f64, cladding: f64, theta: f64) -> f64 {
    1e3 * l * cladding * theta.cos() / (n_g - cladding * theta.sin()) * PI / 180.0
}

/// A figure that may have no value, shown as "none" then.
fn optional(label: &str, value: Option<f64>, digits: usize, unit: &str, note: &str) -> Figure {
    match value.filter(|v| v.is_finite()) {
        Some(v) => figure(label, v, digits, unit, note),
        None => Figure {
            label: label.to_owned(),
            value: None,
            text: "none".to_owned(),
            unit: String::new(),
            note: note.to_owned(),
        },
    }
}

/// A grey dashed line from (x0, y) to (x1, y).
fn level(label: &str, x0: f64, x1: f64, y: f64) -> Curve {
    Curve {
        label: label.to_owned(),
        points: (0..=20)
            .map(|k| [x0 + (x1 - x0) * f64::from(k) / 20.0, y])
            .collect(),
        dashed: true,
        colour: Some(GREY),
    }
}

/// `n + 1` values from `a` to `b`.
fn span(a: f64, b: f64, n: u32) -> impl Iterator<Item = f64> {
    (0..=n).map(move |k| a + (b - a) * f64::from(k) / f64::from(n))
}

/// The x where `f` crosses `target` between `a` and `b` (f(a) and f(b) on either side), bisected
/// to 1e-9.
fn crossing(
    f: impl Fn(f64) -> photonoxide::Result<f64>,
    target: f64,
    (mut a, mut b): (f64, f64),
) -> photonoxide::Result<Option<f64>> {
    let (fa, fb) = (f(a)? - target, f(b)? - target);
    if fa * fb > 0.0 {
        return Ok(None);
    }
    for _ in 0..60 {
        let m = 0.5 * (a + b);
        if (f(m)? - target) * fa > 0.0 {
            a = m;
        } else {
            b = m;
        }
    }
    Ok(Some(0.5 * (a + b)))
}

/// The maximum of a unimodal `f` on [a, b] by golden-section search: (x, f(x)).
fn golden(f: impl Fn(f64) -> f64, (mut a, mut b): (f64, f64)) -> (f64, f64) {
    let r = 0.5 * (5f64.sqrt() - 1.0);
    let (mut c, mut d) = (b - r * (b - a), a + r * (b - a));
    let (mut fc, mut fd) = (f(c), f(d));
    for _ in 0..50 {
        if fc > fd {
            b = d;
            (d, fd) = (c, fc);
            c = b - r * (b - a);
            fc = f(c);
        } else {
            a = c;
            (c, fc) = (d, fd);
            d = a + r * (b - a);
            fd = f(d);
        }
    }
    let x = 0.5 * (a + b);
    (x, f(x))
}

/// A grating whose scattering rate is `alpha(z, power left)` from z = 0, against a fibre's
/// Gaussian field exp(−(z − c)²/w²) on the surface (w the footprint's half-width), phase matched:
/// the share of the fibre's power it couples, (∫ √(2αP) G dz)² over ∫ G² dz on the whole line,
/// with dP/dz = −2αP from P(0) = 1, summed on steps of w/400 until P falls below 1e-9 or 8 w
/// past c. Also the rates along z, every `every` steps.
fn couple(
    w: f64,
    c: f64,
    alpha: impl Fn(f64, f64, f64) -> f64,
    every: usize,
) -> (f64, Vec<[f64; 2]>) {
    let dz = w / STEPS;
    // the Gaussian normalized on the whole line: ∫ G² dz = 1
    let norm = (2.0 / PI).sqrt() / w;
    let g = |z: f64| norm.sqrt() * (-((z - c) / w).powi(2)).exp();
    let (mut p, mut sum, mut z, mut k) = (1.0f64, 0.0, 0.5 * dz, 0usize);
    let mut rates = Vec::new();
    while p > 1e-9 && z < c + 8.0 * w {
        let gz = g(z);
        let a = alpha(z, gz * gz, p);
        if k % every == 0 {
            rates.push([z, a]);
        }
        // the field the step leaves, √(2αP), with P at the step's middle: P e^(−α dz)
        sum += (2.0 * a * p * (-a * dz).exp()).sqrt() * gz * dz;
        p *= (-2.0 * a * dz).exp();
        z += dz;
        k += 1;
    }
    (sum * sum, rates)
}

/// The uniform grating's overlap: leakage α, footprint w, the beam's axis c from the start.
fn uniform(alpha: f64, w: f64, c: f64) -> f64 {
    couple(w, c, |_, _, _| alpha, usize::MAX).0
}

/// The uniform grating at `alpha`, the fibre's axis placed where it couples most: (c, η).
fn uniform_best(alpha: f64, w: f64) -> (f64, f64) {
    golden(|c| uniform(alpha, w, c), (0.0, 3.0 * w))
}

/// The leakage that couples most, and its best offset and overlap: (α, c, η).
fn uniform_optimum(w: f64) -> (f64, f64, f64) {
    let (alpha, eta) = golden(|a| uniform_best(a, w).1, (0.2 / w, 2.0 / w));
    (alpha, uniform_best(alpha, w).0, eta)
}

/// The apodized grating with its scattering capped at `cap`: α(z) = G²/(2P) while that is below
/// the cap (Taillaert et al.'s Eq. 2.1 written with the power left), the cap after it.
fn apodized(w: f64, c: f64, cap: f64, every: usize) -> (f64, Vec<[f64; 2]>) {
    couple(w, c, |_, g2, p| (0.5 * g2 / p).min(cap), every)
}

/// The fibre's footprint half-width on the surface, µm: the waist MFD/2 over cos θ.
fn footprint(mfd: f64, theta: f64) -> f64 {
    0.5 * mfd / theta.cos()
}

/// The specs of the grating coupler's charts.
pub fn specs() -> Vec<ChartSpec> {
    let mfd = |default: f64| {
        param(
            "mfd",
            "Mode field diameter",
            "um",
            (3.0, 20.0, default),
            0.1,
            false,
            "The fibre mode's diameter where its field falls to 1/e: 10.4 µm for a standard single-mode fibre at 1.55 µm, 4 and 6 µm for Passoni et al.'s small-core fibres.",
        )
    };
    let angle = || {
        param(
            "angle",
            "Angle $\\theta$ (°)",
            "",
            (0.0, 30.0, 10.0),
            0.1,
            false,
            "The fibre's angle from the surface's normal, measured in the cladding, degrees, leaning along the guide's direction.",
        )
    };
    let cladding = || {
        param(
            "cladding",
            "Cladding $n_c$",
            "",
            (1.0, 1.5, OXIDE),
            0.001,
            false,
            "The index over the grating: 1 for air, 1.444 for an oxide cladding.",
        )
    };
    let etch = || {
        param(
            "etch",
            "Etch depth $e$",
            "nm",
            (10.0, 180.0, 70.0),
            1.0,
            false,
            "How deep the grooves are etched into the 220 nm film.",
        )
    };
    let fill = || {
        param(
            "fill",
            "Fill $f$",
            "",
            (0.1, 0.9, 0.5),
            0.01,
            false,
            "The unetched tooth's share of the period.",
        )
    };
    vec![
        ChartSpec {
            id: "grating-angle",
            title: "The angle a grating sends light out at",
            about: "A shallow-etched grating in a 220 nm silicon film: the angle from the normal at which its first diffraction order leaves, n_c sin θ = n_eff − λ/Λ, against the wavelength. Positive angles lean along the guide; at θ = 0 the second order reflects the light back down the guide.",
            computed_by: "mode::slab::Slab::modes: TE₀ of the unetched and the etched film, weighted by the fill",
            params: vec![
                param(
                    "period",
                    "Period $\\Lambda$",
                    "nm",
                    (400.0, 1000.0, 630.0),
                    1.0,
                    false,
                    "The grating's period.",
                ),
                etch(),
                fill(),
                cladding(),
            ],
        },
        ChartSpec {
            id: "grating-overlap",
            title: "How much of a fibre's mode an exponential beam catches",
            about: "A uniform grating leaks its light out as e^(−αz) from its start; the share of a fibre's Gaussian mode that field couples, against the leakage α, the fibre's axis placed where it couples most. The line marks the chart's α.",
            computed_by: "the overlap integral on the surface, phase matched, summed on steps of w/400; Tamir and Peng 1977's Eq. 6a for comparison",
            params: vec![
                mfd(10.4),
                angle(),
                param(
                    "leakage",
                    "Leakage $\\alpha$ (1/µm)",
                    "",
                    (0.01, 1.0, 0.13),
                    0.001,
                    true,
                    "The rate at which the grating's field decays along it, per µm; its power decays at 2α.",
                ),
            ],
        },
        ChartSpec {
            id: "grating-apodization",
            title: "Apodizing a grating to the fibre's Gaussian",
            about: "The scattering α(z) along an apodized grating that emits the fibre's Gaussian, 2α = G²/(1 − ∫G²), up to the strongest scattering the grating can give, and the uniform grating's best constant α for comparison.",
            computed_by: "Taillaert et al. 2006's Eq. 2.1 with the power left marched along z, the overlap summed on steps of w/400",
            params: vec![
                mfd(10.4),
                angle(),
                param(
                    "cap",
                    "Strongest $\\alpha$ (1/µm)",
                    "",
                    (0.05, 2.0, 0.3),
                    0.001,
                    true,
                    "The strongest scattering the grating can give, per µm: deeper or wider grooves scatter more.",
                ),
            ],
        },
        ChartSpec {
            id: "grating-directionality",
            title: "How much of a grating's light goes up",
            about: "The share of the light a grating scatters that leaves upwards, against the buried oxide's thickness: the downward half reflects off the substrate and returns in or out of step with the upward half. Mirror pairs of silicon and oxide under the buried oxide send almost all of it up.",
            computed_by: "mode::multilayer::Multilayer::reflection (Chilwell and Hodgkinson's Eq. 13) above and below a sheet at the grooves' mid-depth",
            params: vec![
                param(
                    "box",
                    "Buried oxide $h$",
                    "um",
                    (0.2, 3.0, 2.0),
                    0.005,
                    false,
                    "The buried oxide's thickness between the film and the substrate.",
                ),
                cladding(),
                angle(),
                etch(),
                param(
                    "mirror",
                    "Mirror pairs",
                    "",
                    (0.0, 4.0, 0.0),
                    1.0,
                    false,
                    "Pairs of quarter-wave silicon and oxide layers under the buried oxide.",
                ),
            ],
        },
        ChartSpec {
            id: "grating-band",
            title: "A grating coupler's band",
            about: "The coupling against wavelength, relative to its peak at 1.55 µm, of a grating phase matched there to a fibre at θ: the beam's wavevectors along the surface spread by about 2/w, and away from 1.55 µm the grating's no longer match them.",
            computed_by: "mode::slab::Slab::modes for the grating's effective index at each wavelength; exp(−q²w²/4) for the Gaussian's overlap with a mismatch q",
            params: vec![mfd(10.4), angle(), cladding(), etch(), fill()],
        },
    ]
}

/// The angle the first order leaves at against the wavelength.
pub fn angle(
    period_nm: f64,
    etch_nm: f64,
    fill: f64,
    cladding: f64,
) -> photonoxide::Result<ChartData> {
    let g = Grating {
        etch: etch_nm * 1e-3,
        fill,
        cladding,
    };
    let p = period_nm * 1e-3;
    let theta = |l: f64| -> photonoxide::Result<Option<f64>> {
        let s = (g.index(l)? - l / p) / cladding;
        Ok((s.abs() < 1.0).then(|| s.asin().to_degrees()))
    };
    let (from, to) = (1.45, 1.65);
    let mut points = Vec::new();
    for l in span(from, to, 80) {
        if let Some(t) = theta(l)? {
            points.push([l, t]);
        }
    }
    let mut series = vec![Curve {
        label: format!("θ, Λ = {} nm", sig(period_nm, 4)),
        points,
        dashed: false,
        colour: Some(0),
    }];
    series.push(level("vertical", from, to, 0.0));
    let (tooth, groove) = g.indices(CENTRE)?;
    let n = g.index(CENTRE)?;
    let n_g = g.group_index(CENTRE)?;
    let at = theta(CENTRE)?;
    // the second-order Bragg reflection, where the first order leaves vertically: λ = n_eff Λ
    let vertical = crossing(|l| Ok(g.index(l)? * p - l), 0.0, (1.2, 2.0))?;
    let ten = 10f64.to_radians();
    let figures = vec![
        figure("$n_\\text{eff}$, the film", tooth, 5, "", "220 nm, TE₀"),
        figure(
            "$n_\\text{eff}$, the grooves",
            groove,
            5,
            "",
            &format!("{} nm left", sig(1e3 * (FILM - g.etch), 4)),
        ),
        figure(
            "$n_\\text{eff}$, the grating",
            n,
            5,
            "",
            "the mean, weighted by the fill",
        ),
        optional("θ at 1.55 µm", at, 4, "°", "the first order"),
        optional(
            "band shift per degree",
            at.map(|t| nm_per_degree(CENTRE, n_g, cladding, t.to_radians())),
            3,
            "nm",
            "dλ/dθ at 1.55 µm",
        ),
        figure(
            "period for 10° at 1.55 µm",
            1e3 * CENTRE / (n - cladding * ten.sin()),
            4,
            "nm",
            "",
        ),
        optional(
            "vertical at",
            vertical.map(|l| 1e3 * l),
            5,
            "nm",
            "λ = n_eff Λ: reflected back",
        ),
    ];
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "wavelength λ (µm)".to_owned(),
        x_length: false,
        y_label: "angle θ (°)".to_owned(),
        y_log: false,
        y_range: None,
        series,
        marker: Some(CENTRE),
        figures,
        note: format!(
            "Exact slab modes, no grid: TE₀ of 220 nm and of {} nm of silicon (3.473) on oxide (1.444) under {}, their indices weighted {} : {}, a first-order estimate of the grating's; the indices held fixed over wavelength.",
            sig(1e3 * (FILM - g.etch), 4),
            sig(cladding, 4),
            sig(fill, 3),
            sig(1.0 - fill, 3)
        ),
    })
}

/// A uniform grating's overlap with the fibre against its leakage.
pub fn overlap(mfd: f64, angle_deg: f64, leakage: f64) -> photonoxide::Result<ChartData> {
    let theta = angle_deg.to_radians();
    let w = footprint(mfd, theta);
    let (best, offset, eta) = uniform_optimum(w);
    let top = 3.0 * best.max(leakage);
    let points: Vec<[f64; 2]> = span(top / 120.0, top, 119)
        .map(|a| [a, uniform_best(a, w).1])
        .collect();
    let (at_offset, at) = uniform_best(leakage, w);
    let series = vec![Curve {
        label: "uniform grating".to_owned(),
        points,
        dashed: false,
        colour: Some(0),
    }];
    let figures = vec![
        figure(
            "best overlap",
            100.0 * eta,
            4,
            "%",
            "Tamir and Peng: at most 80 %",
        ),
        figure("at α", best, 4, "1/µm", ""),
        figure(
            "$\\alpha\\,\\mathrm{MFD}\\sec\\theta$",
            best * mfd / theta.cos(),
            3,
            "",
            "Tamir and Peng's Eq. 6a: 1.36",
        ),
        figure(
            "fibre's axis from the start",
            offset,
            3,
            "µm",
            "along the surface",
        ),
        figure("overlap at the chart's α", 100.0 * at, 4, "%", ""),
        figure("its axis from the start", at_offset, 3, "µm", ""),
    ];
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "leakage α (1/µm)".to_owned(),
        x_length: false,
        y_label: "overlap with the fibre".to_owned(),
        y_log: false,
        y_range: Some([0.0, 1.0]),
        series,
        marker: Some(leakage),
        figures,
        note: format!(
            "The field e^(−αz) from the grating's start against the fibre's Gaussian on the surface, its footprint w = MFD/(2 cos θ) = {} µm, phase matched; the fibre's axis placed where it couples most; summed on steps of w/400.",
            sig(w, 4)
        ),
    })
}

/// The apodized grating's scattering along z, capped, against the uniform grating's: the grating
/// starts 2.5 w before the fibre's axis, where the Gaussian's power has fallen to e^(−12.5).
pub fn apodization(mfd: f64, angle_deg: f64, cap: f64) -> photonoxide::Result<ChartData> {
    let theta = angle_deg.to_radians();
    let w = footprint(mfd, theta);
    let (best, _, eta_uniform) = uniform_optimum(w);
    let c = 2.5 * w;
    let (eta, rates) = apodized(w, c, cap, 8);
    let end = 3.0 * w;
    let points: Vec<[f64; 2]> = rates
        .into_iter()
        .map(|[z, a]| [z - c, a])
        .filter(|r| r[0] <= end)
        .collect();
    let axis = points
        .iter()
        .find(|r| r[0] >= 0.0)
        .map_or(f64::NAN, |r| r[1]);
    let capped = points
        .iter()
        .find(|r| r[1] >= cap * (1.0 - 1e-12))
        .map(|r| r[0]);
    let series = vec![
        Curve {
            label: "apodized".to_owned(),
            points,
            dashed: false,
            colour: Some(0),
        },
        level("uniform, its best α", -c, end, best),
    ];
    let figures = vec![
        figure(
            "apodized overlap",
            100.0 * eta,
            4,
            "%",
            "capped at the strongest α",
        ),
        figure("uniform's best", 100.0 * eta_uniform, 4, "%", ""),
        figure("α at the fibre's axis", axis, 3, "1/µm", ""),
        optional(
            "the cap reached at",
            capped,
            3,
            "µm",
            "past the fibre's axis",
        ),
        figure("footprint w", w, 3, "µm", "MFD/(2 cos θ)"),
    ];
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "along the grating from the fibre's axis z (µm)".to_owned(),
        x_length: false,
        y_label: "scattering α (1/µm)".to_owned(),
        y_log: false,
        y_range: Some([0.0, 1.05 * cap.max(best)]),
        series,
        marker: Some(0.0),
        figures,
        note: format!(
            "α(z) = G²/(2P) with the power left P marched along z (Taillaert et al.'s Eq. 2.1), capped at {} per µm; the fibre's Gaussian on the surface, footprint w = {} µm, its axis at the line, the grating from 2.5 w before it; summed on steps of w/400.",
            sig(cap, 3),
            sig(w, 4)
        ),
    })
}
/// The upward share of a sheet at the grooves' mid-depth, a film over `h` µm of oxide over
/// `pairs` quarter-wave pairs over silicon, at the in-plane index `s` (n_c sin θ): (up, |r_b|²).
pub fn upward(
    h: f64,
    cladding: f64,
    s: f64,
    etch: f64,
    pairs: usize,
) -> photonoxide::Result<(f64, f64)> {
    let l = Wavelength::um(CENTRE)?;
    let n = |x: f64| c64::new(x, 0.0);
    let inside = (s / SILICON).asin();
    let top = Multilayer::new(
        n(SILICON),
        &[(n(SILICON), Length::um(0.5 * etch))],
        n(cladding),
    )?;
    let rt = top.reflection(Polarization::Te, l, inside)?.r;
    let quarter = |m: f64| CENTRE / (4.0 * (m * m - s * s).sqrt());
    let mut films = vec![
        (n(SILICON), Length::um(FILM - 0.5 * etch)),
        (n(OXIDE), Length::um(h)),
    ];
    for _ in 0..pairs {
        films.push((n(SILICON), Length::um(quarter(SILICON))));
        films.push((n(OXIDE), Length::um(quarter(OXIDE))));
    }
    let bottom = Multilayer::new(n(SILICON), &films, n(SILICON))?;
    let rb = bottom.reflection(Polarization::Te, l, inside)?.r;
    let up = (1.0 + rb).norm_sqr() * (1.0 - rt.norm_sqr());
    let down = (1.0 + rt).norm_sqr() * (1.0 - rb.norm_sqr());
    Ok((up / (up + down), rb.norm_sqr()))
}

/// The upward share against the buried oxide's thickness.
pub fn directionality(
    h: f64,
    cladding: f64,
    angle_deg: f64,
    etch_nm: f64,
    pairs: usize,
) -> photonoxide::Result<ChartData> {
    let s = cladding * angle_deg.to_radians().sin();
    let e = etch_nm * 1e-3;
    let curve = |m: usize| -> photonoxide::Result<Vec<[f64; 2]>> {
        span(0.2, 3.0, 280)
            .map(|x| Ok([x, upward(x, cladding, s, e, m)?.0]))
            .collect()
    };
    let mut series = vec![Curve {
        label: if pairs == 0 {
            "upward share".to_owned()
        } else {
            format!("upward share, {pairs} mirror pairs")
        },
        points: curve(pairs)?,
        dashed: false,
        colour: Some(0),
    }];
    if pairs > 0 {
        series.push(Curve {
            label: "no mirror".to_owned(),
            points: curve(0)?,
            dashed: true,
            colour: Some(GREY),
        });
    }
    let (up, rb2) = upward(h, cladding, s, e, pairs)?;
    // the oscillation's period in h: half a wavelength across the oxide, λ/(2√(n_ox² − s²))
    let period = CENTRE / (2.0 * (OXIDE * OXIDE - s * s).sqrt());
    let near = (h - 0.5 * period).max(0.05);
    let (best_h, best) = span(near, near + period, 400)
        .map(|x| upward(x, cladding, s, e, pairs).map(|u| (x, u.0)))
        .collect::<photonoxide::Result<Vec<_>>>()?
        .into_iter()
        .fold(
            (f64::NAN, f64::NEG_INFINITY),
            |a, b| if b.1 > a.1 { b } else { a },
        );
    let ceiling = uniform_optimum(1.0).2;
    let figures = vec![
        figure("upward share", 100.0 * up, 4, "%", ""),
        figure(
            "× the uniform grating's best overlap",
            100.0 * up * ceiling,
            3,
            "%",
            "an estimate of its efficiency",
        ),
        figure(
            "substrate's reflectance",
            100.0 * rb2,
            4,
            "%",
            "seen from the film",
        ),
        figure(
            "best oxide nearby",
            best_h,
            3,
            "µm",
            &format!("{} % up", sig(100.0 * best, 3)),
        ),
        figure("period in h", period, 4, "µm", "λ/(2 n_ox cos θ_ox)"),
    ];
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "buried oxide h".to_owned(),
        x_length: true,
        y_label: "share of the light sent up".to_owned(),
        y_log: false,
        y_range: Some([0.0, 1.0]),
        series,
        marker: Some(h),
        figures,
        note: format!(
            "Exact transfer matrices, no grid, at 1.55 µm, TE: a sheet {} nm below the film's top radiating equally up and down, between the reflections of the {} nm of silicon and the cladding ({}) above it and of the rest of the film, the oxide, any mirror pairs and the silicon substrate below. A model of where the light goes, not the grating's full solution.",
            sig(0.5 * etch_nm, 3),
            sig(0.5 * etch_nm, 3),
            sig(cladding, 4)
        ),
    })
}

/// The coupling against wavelength from the beam's spread of wavevectors.
pub fn band(
    mfd: f64,
    angle_deg: f64,
    cladding: f64,
    etch_nm: f64,
    fill: f64,
) -> photonoxide::Result<ChartData> {
    let g = Grating {
        etch: etch_nm * 1e-3,
        fill,
        cladding,
    };
    let theta = angle_deg.to_radians();
    let s = cladding * theta.sin();
    let n = g.index(CENTRE)?;
    let period = CENTRE / (n - s);
    let w = footprint(mfd, theta);
    let db = |l: f64| -> photonoxide::Result<f64> {
        let q = 2.0 * PI * (g.index(l)? - s) / l - 2.0 * PI / period;
        // exp(−q²w²/4) in dB
        Ok(-10.0 / std::f64::consts::LN_10 * q * q * w * w / 4.0)
    };
    let (from, to) = (1.35, 1.75);
    let mut points = Vec::new();
    for l in span(from, to, 160) {
        let v = db(l)?;
        if v >= -6.0 {
            points.push([l, v]);
        }
    }
    let width = |level: f64| -> photonoxide::Result<Option<f64>> {
        Ok(
            match (
                crossing(db, level, (from, CENTRE))?,
                crossing(db, level, (CENTRE, to))?,
            ) {
                (Some(a), Some(b)) => Some(1e3 * (b - a)),
                _ => None,
            },
        )
    };
    let n_g = g.group_index(CENTRE)?;
    // the closed form: 1 dB where qw = 2√(0.1 ln 10), q changing by 2π(n_g − n_c sin θ)/λ² per µm
    let estimate = 1e3 * 2.0 * (0.1 * std::f64::consts::LN_10).sqrt() * 2.0 * CENTRE * CENTRE
        / (2.0 * PI * (n_g - s) * w);
    let series = vec![
        Curve {
            label: "coupling, relative".to_owned(),
            points,
            dashed: false,
            colour: Some(0),
        },
        level("1 dB down", from, to, -1.0),
    ];
    let figures = vec![
        optional(
            "1 dB bandwidth",
            width(-1.0)?,
            4,
            "nm",
            "wider than the chart if none",
        ),
        figure(
            "its closed form",
            estimate,
            4,
            "nm",
            "0.306 λ² cos θ/((n_g − n_c sin θ) w₀)",
        ),
        optional("3 dB bandwidth", width(-3.0)?, 4, "nm", ""),
        figure("period", 1e3 * period, 4, "nm", "phase matched at 1.55 µm"),
        figure("$n_\\text{eff}$", n, 5, "", "the grating's, at 1.55 µm"),
        figure("$n_g$", n_g, 4, "", "its group index"),
        figure(
            "band shift per degree",
            nm_per_degree(CENTRE, n_g, cladding, theta),
            3,
            "nm",
            "",
        ),
    ];
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "wavelength λ (µm)".to_owned(),
        x_length: false,
        y_label: "coupling relative to 1.55 µm (dB)".to_owned(),
        y_log: false,
        y_range: Some([-6.0, 0.5]),
        series,
        marker: Some(CENTRE),
        figures,
        note: format!(
            "exp(−q²w²/4), q = 2π(n_eff(λ) − n_c sin θ)/λ − 2π/Λ, the fibre's footprint w = {} µm; n_eff from the exact slab modes of 220 and {} nm of silicon on oxide weighted by the fill, the indices held fixed over wavelength. Only the finite beam's broadening: the grating's own resonance and its directionality's change with λ are left out.",
            sig(w, 4),
            sig(1e3 * (FILM - g.etch), 4)
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "{a} against {b}");
    }

    fn value(data: &ChartData, label: &str) -> f64 {
        data.figures
            .iter()
            .find(|f: &&Figure| f.label == label)
            .and_then(|f| f.value)
            .unwrap_or_else(|| panic!("no figure {label}"))
    }

    /// A uniform grating couples at most 80 % of a Gaussian, at α MFD sec θ = 1.36 (Tamir and
    /// Peng 1977, p. 237, Eqs. 6a): the overlap's own maximum, whatever the fibre.
    #[test]
    fn the_uniform_grating_couples_at_most_tamir_and_pengs_80_percent() {
        for (mfd, angle) in [(10.4, 10.0), (4.0, 0.0), (6.0, 20.0)] {
            let data = overlap(mfd, angle, 0.13).unwrap();
            close(value(&data, "best overlap"), 80.1, 0.05);
            close(
                value(&data, "$\\alpha\\,\\mathrm{MFD}\\sec\\theta$"),
                1.36,
                0.01,
            );
        }
        // Chen et al. 2010: α ≈ 0.13 per µm for a 10.4 µm fibre
        close(
            value(&overlap(10.4, 10.0, 0.13).unwrap(), "at α"),
            0.13,
            0.002,
        );
    }

    /// The marched overlap of e^(−αz) with a Gaussian is the integral's, summed independently on
    /// a step a hundred times finer with the normalizations ∫ e^(−2αz) = 1/(2α) and
    /// ∫ e^(−2(z − c)²/w²) = w √(π/2) written out.
    #[test]
    fn the_overlap_is_the_integral() {
        let (a, w, c) = (0.2, 3.0, 2.5);
        // an independent sum on a finer step, with the normalizations written out
        let n = 400_000;
        let (lo, hi) = (0.0, c + 12.0 * w);
        let dz = (hi - lo) / f64::from(n);
        let mut fg = 0.0;
        for k in 0..n {
            let z = lo + (f64::from(k) + 0.5) * dz;
            fg += (-a * z).exp() * (-((z - c) / w).powi(2)).exp() * dz;
        }
        let exact = fg * fg / ((0.5 / a) * w * (PI / 2.0).sqrt());
        close(uniform(a, w, c), exact, 2e-4);
    }

    /// Taillaert et al. 2006: the apodized grating couples more than the uniform's 80 %, and
    /// with no cap all of the Gaussian past its start.
    #[test]
    fn apodization_beats_the_uniform_grating() {
        let data = apodization(10.4, 10.0, 2.0).unwrap();
        assert!(value(&data, "apodized overlap") > 97.0);
        let weak = apodization(10.4, 10.0, 0.13).unwrap();
        let (a, u) = (
            value(&weak, "apodized overlap"),
            value(&weak, "uniform's best"),
        );
        assert!(a > u && a < 97.0, "{a} {u}");
    }

    /// The model against Taillaert et al. 2006's Table I: 37 % in air on 1 µm of oxide, 44 %
    /// under an index-matched cladding, 79 % with a two-pair mirror, and the best oxide near 0.9
    /// and 1.45 µm; each the upward share times the 80 % overlap.
    #[test]
    fn directionality_reproduces_taillaerts_table() {
        let est = |h: f64, nc: f64, deg: f64, pairs: usize| {
            0.801
                * upward(h, nc, nc * deg.to_radians().sin(), 0.07, pairs)
                    .unwrap()
                    .0
        };
        close(est(1.0, 1.0, 10.0, 0), 0.37, 0.01);
        close(est(1.0, 1.46, 8.0, 0), 0.44, 0.015);
        close(est(1.47, 1.46, 8.0, 2), 0.79, 0.01);
        let data = directionality(1.0, 1.46, 8.0, 70.0, 0).unwrap();
        close(value(&data, "best oxide nearby"), 0.9, 0.06);
        let data = directionality(1.45, 1.46, 8.0, 70.0, 0).unwrap();
        close(value(&data, "best oxide nearby"), 1.45, 0.07);
        // the share never exceeds 1, and the period is half a wavelength in the oxide
        close(
            value(&data, "period in h"),
            1.55 / (2.0 * (OXIDE * OXIDE - (1.46 * 8f64.to_radians().sin()).powi(2)).sqrt()),
            1e-9,
        );
    }

    /// Phase matching: the angle on the chart satisfies n_c sin θ = n_eff − λ/Λ with the slab's
    /// indices, Taillaert et al.'s 630 nm grating leaves near their 10° in air, and its band
    /// shift per degree is near their 9 nm.
    #[test]
    fn the_angle_is_phase_matched() {
        let g = Grating {
            etch: 0.07,
            fill: 0.5,
            cladding: 1.0,
        };
        let data = angle(630.0, 70.0, 0.5, 1.0).unwrap();
        for &[l, t] in &data.series[0].points {
            close(t.to_radians().sin(), g.index(l).unwrap() - l / 0.63, 1e-12);
        }
        close(value(&data, "θ at 1.55 µm"), 10.0, 2.5);
        close(value(&data, "band shift per degree"), 9.0, 1.5);
        let vertical = value(&data, "vertical at") * 1e-3;
        close(g.index(vertical).unwrap() * 0.63, vertical, 1e-6);
    }

    /// The band's 1 dB width on the curve is the closed form's, and Taillaert et al.'s grating
    /// in air with a standard fibre has about their 40 nm.
    #[test]
    fn the_band_is_the_closed_form() {
        let data = band(10.4, 10.0, 1.0, 70.0, 0.5).unwrap();
        let (curve, closed) = (
            value(&data, "1 dB bandwidth"),
            value(&data, "its closed form"),
        );
        close(curve, closed, 0.02 * closed);
        close(curve, 40.0, 5.0);
        // a smaller mode field widens the band in proportion
        let small = band(5.2, 10.0, 1.0, 70.0, 0.5).unwrap();
        close(
            value(&small, "its closed form"),
            2.0 * closed,
            1e-6 * closed,
        );
    }
}
