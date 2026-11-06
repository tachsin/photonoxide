//! The plasmonics lesson's charts: a Drude metal's permittivity (`material::Model::Drude`), the
//! surface plasmon of a metal–dielectric interface solved as a TM mode of a planar stack
//! (`mode::multilayer::Multilayer::mode_near`) with its propagation and decay lengths, Mie's
//! efficiencies of a sphere (`fdtd::Mie`), a Drude sphere's localized resonance
//! (`fdtd::Sphere::cross_sections`) beside the quasi-static dipole, and Mie's colloidal gold
//! (`Mie::gold_in_water`) against his table of their absorption. Nothing here has a grid.
//!
//! The metal is a model, ε = ε∞ − ω_p²/(ω² + iγω), with its plasma frequency at f_p = 1 c/µm:
//! a frequency in c/µm is then ω/ω_p, γ/2π in c/µm is γ/ω_p, and a length in µm is in units of
//! the plasma wavelength λ_p = 2πc/ω_p.

use std::f64::consts::PI;

use num_complex::Complex64 as c64;
use photonoxide::fdtd::{Mie, Sphere};
use photonoxide::material::{Material, Model, Provenance};
use photonoxide::mode::Polarization;
use photonoxide::mode::multilayer::Multilayer;
use photonoxide::units::{Frequency, Length, Wavelength, refractive_index};

use super::{ChartData, ChartSpec, Curve, figure, invalid, param, sig};

/// The palette's grey, for reference lines.
const GREY: usize = 7;

/// The quasi-static limit's colour, beside Mie's extinction.
const DIPOLE: usize = 3;

/// A curve in the palette's colour `colour`.
fn curve(label: &str, points: Vec<[f64; 2]>, dashed: bool, colour: usize) -> Curve {
    Curve {
        label: label.to_owned(),
        points,
        dashed,
        colour: Some(colour),
    }
}

/// `n` points from `a` to `b`, both included.
fn span(a: f64, b: f64, n: usize) -> Vec<f64> {
    (0..n)
        .map(|k| {
            if k + 1 == n {
                b
            } else {
                a + (b - a) * k as f64 / (n - 1) as f64
            }
        })
        .collect()
}

/// The lesson's charts.
pub fn specs() -> Vec<ChartSpec> {
    let damping = |default: f64| {
        param(
            "damping",
            "Damping $\\gamma / \\omega_p$",
            "",
            (0.002, 0.3, default),
            0.001,
            true,
            "The electrons' collision rate in units of the plasma frequency: the metal's loss.",
        )
    };
    let medium = |default: f64| {
        param(
            "medium",
            "Dielectric $n_d$",
            "",
            (1.0, 2.0, default),
            0.01,
            false,
            "The refractive index of the lossless dielectric on the metal or round the sphere: 1 for air or vacuum, 1.33 for water, 1.44 for silica.",
        )
    };
    vec![
        ChartSpec {
            id: "plasmon-drude",
            title: "A free-electron metal's permittivity",
            about: "The Drude permittivity ε = ε∞ − ω_p²/(ω² + iγω) against frequency, in units of the plasma frequency: below the frequency where Re ε crosses zero it is negative, and the metal reflects; above, it turns transparent.",
            computed_by: "material::Model::Drude, Material::permittivity, Material::refractive_index",
            params: vec![
                damping(0.02),
                param(
                    "background",
                    "Background $\\varepsilon_\\infty$",
                    "",
                    (1.0, 10.0, 1.0),
                    0.01,
                    false,
                    "The permittivity the bound electrons leave at these frequencies; 1 for free electrons alone.",
                ),
            ],
        },
        ChartSpec {
            id: "plasmon-dispersion",
            title: "The surface plasmon's dispersion",
            about: "The TM mode bound to a flat interface between a Drude metal and a dielectric, solved as a mode of a planar stack: its frequency against the real part of its wavenumber, beside light in the dielectric and light in the metal above its plasma frequency.",
            computed_by: "mode::multilayer::Multilayer::mode_near (TM) on a Drude metal under a dielectric; material::Model::Drude",
            params: vec![medium(1.0), damping(0.01)],
        },
        ChartSpec {
            id: "plasmon-length",
            title: "How far a surface plasmon travels, and how far it reaches",
            about: "The surface plasmon's propagation length, the distance its power falls by e, and the depths its field reaches into the dielectric and into the metal, in units of the plasma wavelength, against frequency up to the surface plasma frequency.",
            computed_by: "mode::multilayer::Multilayer::mode_near (TM), MultilayerMode::effective_index; the decay constants from the effective index",
            params: vec![
                medium(1.0),
                damping(0.01),
                param(
                    "at",
                    "Frequency $\\omega / \\omega_\\text{sp}$",
                    "",
                    (0.05, 0.98, 0.5),
                    0.01,
                    false,
                    "The frequency to read the lengths at, in units of the surface plasma frequency ω_sp = ω_p/√(ε∞ + ε_d).",
                ),
            ],
        },
        ChartSpec {
            id: "mie-efficiency",
            title: "A sphere's extinction, scattering and absorption",
            about: "Mie's efficiencies, the cross-sections over the sphere's shadow πρ², against the size parameter α = 2πn₀ρ/λ, for a sphere of relative index n + iκ; and Rayleigh's small-sphere limit.",
            computed_by: "fdtd::Mie::new, Mie::extinction, Mie::scattering, Mie::absorption",
            params: vec![
                param(
                    "index",
                    "Index $n$",
                    "",
                    (1.01, 4.0, 1.5),
                    0.01,
                    false,
                    "The real part of the sphere's index relative to the medium round it.",
                ),
                param(
                    "kappa",
                    "Extinction $\\kappa$",
                    "",
                    (0.0, 5.0, 0.0),
                    0.01,
                    false,
                    "The imaginary part of the sphere's relative index: 0 for a lossless sphere.",
                ),
                param(
                    "up_to",
                    "Up to $\\alpha$",
                    "",
                    (2.0, 60.0, 30.0),
                    0.5,
                    false,
                    "The largest size parameter drawn.",
                ),
            ],
        },
        ChartSpec {
            id: "mie-plasmon",
            title: "A metal sphere's localized plasmon",
            about: "Mie's extinction, scattering and absorption efficiencies of a Drude metal sphere against frequency, beside the quasi-static dipole's extinction: the resonance near ε = −2ε_d, its shift and its width as the sphere grows.",
            computed_by: "fdtd::Sphere::cross_sections with material::Model::Drude's permittivity; the quasi-static dipole computed in the studio",
            params: vec![
                param(
                    "radius",
                    "Radius $\\rho / \\lambda_p$",
                    "",
                    (0.002, 0.3, 0.02),
                    0.001,
                    true,
                    "The sphere's radius in units of the plasma wavelength λ_p = 2πc/ω_p.",
                ),
                damping(0.03),
                medium(1.0),
            ],
        },
        ChartSpec {
            id: "mie-gold",
            title: "Mie's colloidal gold",
            about: "The light a gold solution of 1 mm³ of gold per litre of water takes from a beam over 1 mm, in thousandths, at Mie's seven wavelengths, for spheres of one diameter: by extinction, by absorption and by scattering, with gold's index from his table; beside his own numbers for the finest particles, and a free-electron metal fitted to his gold at 650 nm.",
            computed_by: "fdtd::Mie::gold_in_water, Mie::new, Mie::extinction, Mie::absorption, Mie::scattering; material::Model::Drude for the fitted metal",
            params: vec![param(
                "diameter",
                "Diameter $2\\rho$",
                "nm",
                (1.0, 200.0, 20.0),
                1.0,
                false,
                "The gold spheres' diameter, nm.",
            )],
        },
    ]
}

/// A Drude metal with its plasma frequency at 1 c/µm, valid far beyond the charts' frequencies.
pub fn drude(background: f64, damping: f64) -> photonoxide::Result<Material> {
    Material::new(
        "Drude metal",
        Model::Drude {
            eps_inf: background,
            plasma: Frequency::natural(1.0)?,
            damping,
        },
        Wavelength::um(1.0 / 100.0)?,
        Wavelength::um(1.0 / 1e-3)?,
        Provenance {
            reference: "ε(ω) = ε∞ − ω_p² / (ω² + iγω)".to_owned(),
            doi: String::new(),
            data: String::new(),
            temperature: None,
            notes: String::new(),
        },
    )
}

/// The metal's permittivity at ω/ω_p = `w`.
fn eps(metal: &Material, w: f64) -> photonoxide::Result<c64> {
    metal.permittivity(Wavelength::um(1.0 / w)?)
}

/// Bisection for the `w` in `(a, b)` where `f` changes sign, `f(a)` and `f(b)` of opposite
/// signs.
fn bisect(
    mut a: f64,
    mut b: f64,
    f: impl Fn(f64) -> photonoxide::Result<f64>,
) -> photonoxide::Result<f64> {
    let fa0 = f(a)?;
    for _ in 0..200 {
        let m = 0.5 * (a + b);
        if m <= a || m >= b {
            break;
        }
        if (f(m)? > 0.0) == (fa0 > 0.0) {
            a = m;
        } else {
            b = m;
        }
    }
    Ok(0.5 * (a + b))
}

/// The Drude permittivity against ω/ω_p: Re ε, Im ε, and the lines ε = −1 and ε = −2 where a
/// flat surface and a sphere in vacuum resonate.
pub fn drude_chart(damping: f64, background: f64) -> photonoxide::Result<ChartData> {
    let metal = drude(background, damping)?;
    let (lo, hi) = (0.2, 2.0);
    let mut re = Vec::new();
    let mut im = Vec::new();
    for w in span(lo, hi, 721) {
        let e = eps(&metal, w)?;
        re.push([w, e.re]);
        im.push([w, e.im]);
    }
    let re_at = |target: f64| {
        bisect(0.01, 1.0 / background.sqrt() + 1.0, |w| {
            Ok(eps(&metal, w)?.re - target)
        })
    };
    let zero = re_at(0.0)?;
    let flat = re_at(-1.0)?;
    let sphere = re_at(-2.0)?;
    let reflectance = |w: f64| -> photonoxide::Result<f64> {
        let n = metal.refractive_index(Wavelength::um(1.0 / w)?)?;
        Ok(((1.0 - n) / (1.0 + n)).norm_sqr())
    };
    let half = eps(&metal, 0.5)?;
    let figures = vec![
        figure(
            "Re $\\varepsilon = 0$ at",
            zero,
            4,
            "ω_p",
            "$\\sqrt{1/\\varepsilon_\\infty - (\\gamma/\\omega_p)^2}$: the metal turns transparent above",
        ),
        figure(
            "Re $\\varepsilon = -1$ at",
            flat,
            4,
            "ω_p",
            "a flat surface in vacuum: the surface plasmon's limit",
        ),
        figure(
            "Re $\\varepsilon = -2$ at",
            sphere,
            4,
            "ω_p",
            "a small sphere in vacuum: its dipole resonance",
        ),
        figure(
            "$\\varepsilon$ at $0.5\\thinspace\\omega_p$",
            half.re,
            4,
            "",
            &format!("$+ {}i$", sig(half.im, 3)),
        ),
        figure(
            "reflectance at $0.5\\thinspace\\omega_p$",
            reflectance(0.5)?,
            4,
            "",
            "normal incidence from vacuum, $\\lvert(1 - n)/(1 + n)\\rvert^2$",
        ),
        figure(
            "reflectance at $1.5\\thinspace\\omega_p$",
            reflectance(1.5)?,
            3,
            "",
            "above the plasma frequency",
        ),
    ];
    let line = |v: f64| vec![[lo, v], [hi, v]];
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "frequency ω/ω_p".to_owned(),
        x_length: false,
        y_label: "permittivity ε".to_owned(),
        y_log: false,
        y_range: Some([-12.0, 6.0]),
        series: vec![
            curve("Re ε", re, false, 0),
            curve("Im ε", im, false, 1),
            curve("ε = −1", line(-1.0), true, GREY),
            curve("ε = −2", line(-2.0), true, GREY),
        ],
        marker: Some(zero),
        figures,
        note: format!(
            "Exact, no grid: ε = {background} − ω_p²/(ω² + iγω) with γ = {damping} ω_p. The line marks Re ε = 0."
        ),
    })
}

/// A flat interface: the dielectric of index `medium` above, as the cover and a film of its own
/// index, over the metal of permittivity `metal` as the substrate.
fn interface(medium: f64, metal: c64) -> photonoxide::Result<Multilayer> {
    let d = c64::new(medium, 0.0);
    Multilayer::new(d, &[(d, Length::um(1.0))], refractive_index(metal))
}

/// The closed form k/k₀ = √(ε_m ε_d/(ε_m + ε_d)), on the branch with Im ≥ 0.
pub fn closed_form(eps_m: c64, eps_d: f64) -> c64 {
    refractive_index(eps_m * eps_d / (eps_m + eps_d))
}

/// A surface plasmon at one frequency: ω/ω_p and its effective index k/k₀.
#[derive(Clone, Copy, Debug)]
pub struct Plasmon {
    pub w: f64,
    pub eps_m: c64,
    pub index: c64,
}

/// The surface plasmon of a Drude metal (ε∞ = 1) under a dielectric of index `medium`, solved by
/// the stack's TM secant iteration from the closed form put 0.1 % off, at every frequency of
/// `ws` where it is bound (Re k above the dielectric's light line).
pub fn plasmons(medium: f64, damping: f64, ws: &[f64]) -> photonoxide::Result<Vec<Plasmon>> {
    let metal = drude(1.0, damping)?;
    let mut out = Vec::new();
    for &w in ws {
        let eps_m = eps(&metal, w)?;
        let guess = closed_form(eps_m, medium * medium) * 1.001;
        let Ok(mode) =
            interface(medium, eps_m)?.mode_near(Polarization::Tm, Wavelength::um(1.0 / w)?, guess)
        else {
            break;
        };
        let index = mode.effective_index();
        if index.re <= medium * (1.0 + 1e-9) || index.im < 0.0 {
            break;
        }
        out.push(Plasmon { w, eps_m, index });
    }
    Ok(out)
}

/// The surface plasma frequency of a Drude metal with ε∞ = 1 and no loss, ω_p/√(1 + ε_d).
fn surface_frequency(medium: f64) -> f64 {
    1.0 / (1.0 + medium * medium).sqrt()
}

/// The frequencies a surface plasmon is followed at: evenly to 0.9 ω_sp, then closer as it
/// bends, to ω_sp and past it while it stays bound.
fn plasmon_frequencies(medium: f64, damping: f64) -> Vec<f64> {
    let sp = surface_frequency(medium);
    let mut ws = span(0.02, 0.9 * sp, 300);
    ws.pop();
    ws.extend(span(0.9 * sp, (sp * (1.0 + 4.0 * damping)).min(0.999), 600));
    ws
}

/// The surface plasmon's dispersion: ω/ω_p against Re k c/ω_p.
pub fn dispersion(medium: f64, damping: f64) -> photonoxide::Result<ChartData> {
    let sp = surface_frequency(medium);
    let found = plasmons(medium, damping, &plasmon_frequencies(medium, damping))?;
    if found.is_empty() {
        return Err(invalid(
            "plasmon-dispersion",
            "no bound surface plasmon found",
        ));
    }
    let points: Vec<[f64; 2]> = found.iter().map(|p| [p.index.re * p.w, p.w]).collect();
    let (k_max, w_bend) = points
        .iter()
        .fold((0.0, 0.0), |a, p| if p[0] > a.0 { (p[0], p[1]) } else { a });
    let worst = found
        .iter()
        .map(|p| (p.index - closed_form(p.eps_m, medium * medium)).norm() / p.index.norm())
        .fold(0.0, f64::max);
    let half = found
        .iter()
        .min_by(|a, b| (a.w - 0.5 * sp).abs().total_cmp(&(b.w - 0.5 * sp).abs()))
        .copied()
        .ok_or_else(|| invalid("plasmon-dispersion", "no plasmon at half ω_sp"))?;
    let metal = drude(1.0, damping)?;
    let mut bulk = Vec::new();
    for w in span(1.0, 1.6, 121) {
        let n = metal.refractive_index(Wavelength::um(1.0 / w)?)?;
        bulk.push([n.re * w, w]);
    }
    let x_end = (1.15 * k_max).max(1.7);
    let figures = vec![
        figure(
            "$\\omega_\\text{sp}$",
            sp,
            4,
            "ω_p",
            "$\\omega_p/\\sqrt{1 + \\varepsilon_d}$, where Re $\\varepsilon_m = -\\varepsilon_d$ without loss",
        ),
        figure(
            "largest Re $k$",
            k_max,
            4,
            "ω_p/c",
            &format!(
                "at $\\omega = {}\\thinspace\\omega_p$, where the loss bends it back",
                sig(w_bend, 4)
            ),
        ),
        figure(
            "$k/k_d$ at $\\omega_\\text{sp}/2$",
            half.index.re / medium,
            5,
            "",
            "how far past the dielectric's light line",
        ),
        figure(
            "against the closed form",
            worst,
            2,
            "",
            "largest relative difference of the stack's mode from $\\sqrt{\\varepsilon_m\\varepsilon_d/(\\varepsilon_m + \\varepsilon_d)}$",
        ),
    ];
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "wavenumber Re k, in ω_p/c".to_owned(),
        x_length: false,
        y_label: "frequency ω/ω_p".to_owned(),
        y_log: false,
        y_range: Some([0.0, 1.6]),
        series: vec![
            curve("surface plasmon (TM)", points, false, 0),
            curve(
                "light in the dielectric",
                vec![[0.0, 0.0], [x_end, x_end / medium]],
                true,
                GREY,
            ),
            curve("light in the metal", bulk, false, 1),
            curve("ω_sp", vec![[0.0, sp], [x_end, sp]], true, 2),
        ],
        marker: None,
        figures,
        note: format!(
            "Exact, no grid: the TM mode of a dielectric of index {medium} over a Drude metal, ε_m = 1 − ω_p²/(ω² + iγω) with γ = {damping} ω_p, by the transfer matrix's secant iteration, at {} frequencies. Light in the metal is Re √ε_m ω/c above ω_p.",
            found.len()
        ),
    })
}

/// A surface plasmon's lengths in units of λ_p: its propagation length 1/(2 Im k) and the
/// depths 1/Im k_x its field reaches into the dielectric and the metal.
#[derive(Clone, Copy, Debug)]
pub struct Lengths {
    pub propagation: f64,
    pub dielectric: f64,
    pub metal: f64,
}

/// The lengths of a plasmon in a dielectric of permittivity `eps_d`.
pub fn lengths(p: &Plasmon, eps_d: f64) -> Lengths {
    let k0 = 2.0 * PI * p.w;
    let beta2 = p.index * p.index;
    // the decaying root of k_x² = (ε − β²) k₀²
    let across = |e: c64| refractive_index(e - beta2).im * k0;
    Lengths {
        propagation: 1.0 / (2.0 * p.index.im * k0),
        dielectric: 1.0 / across(c64::new(eps_d, 0.0)),
        metal: 1.0 / across(p.eps_m),
    }
}

/// The surface plasmon's propagation length and decay depths against ω/ω_p.
pub fn length(medium: f64, damping: f64, at: f64) -> photonoxide::Result<ChartData> {
    let sp = surface_frequency(medium);
    let eps_d = medium * medium;
    let mut ws = span(0.05 * sp, 0.995 * sp, 400);
    ws.push(at * sp);
    ws.sort_by(f64::total_cmp);
    let found = plasmons(medium, damping, &ws)?;
    let mut prop = Vec::new();
    let mut diel = Vec::new();
    let mut met = Vec::new();
    let mut wave = Vec::new();
    for p in &found {
        let l = lengths(p, eps_d);
        prop.push([p.w, l.propagation]);
        diel.push([p.w, l.dielectric]);
        met.push([p.w, l.metal]);
        wave.push([p.w, 1.0 / p.w]);
    }
    let here = found
        .iter()
        .find(|p| p.w == at * sp)
        .ok_or_else(|| invalid("plasmon-length", "the plasmon isn't bound there"))?;
    let l = lengths(here, eps_d);
    let figures = vec![
        figure(
            "propagation length",
            l.propagation,
            4,
            "λ_p",
            &format!("at $\\omega = {}\\thinspace\\omega_p$", sig(here.w, 4)),
        ),
        figure(
            "depth in the dielectric",
            l.dielectric,
            4,
            "λ_p",
            "where the field falls by $e$",
        ),
        figure("depth in the metal", l.metal, 4, "λ_p", "the skin depth"),
        figure(
            "wavelength in vacuum",
            1.0 / here.w,
            4,
            "λ_p",
            "$\\lambda = \\lambda_p\\thinspace\\omega_p/\\omega$",
        ),
        figure(
            "propagation over depth",
            l.propagation / l.dielectric,
            4,
            "",
            "how far it travels for how tightly it is held",
        ),
    ];
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "frequency ω/ω_p".to_owned(),
        x_length: false,
        y_label: "length, in λ_p".to_owned(),
        y_log: true,
        y_range: None,
        series: vec![
            curve("propagation length", prop, false, 0),
            curve("depth in the dielectric", diel, false, 1),
            curve("depth in the metal", met, false, 2),
            curve("wavelength in vacuum", wave, true, GREY),
        ],
        marker: Some(here.w),
        figures,
        note: format!(
            "Exact, no grid: the TM mode of a dielectric of index {medium} over a Drude metal (ε∞ = 1, γ = {damping} ω_p), by the transfer matrix; lengths in units of the plasma wavelength λ_p = 2πc/ω_p. The line marks the chosen frequency."
        ),
    })
}

/// Rayleigh's small-sphere scattering, (8/3) α⁴ |(m² − 1)/(m² + 2)|².
fn rayleigh(size: f64, m: c64) -> f64 {
    let m2 = m * m;
    8.0 / 3.0 * size.powi(4) * ((m2 - 1.0) / (m2 + 2.0)).norm_sqr()
}

/// Mie's efficiencies against the size parameter.
pub fn efficiency(index: f64, kappa: f64, up_to: f64) -> photonoxide::Result<ChartData> {
    let m = c64::new(index, kappa);
    let mut ext = Vec::new();
    let mut sca = Vec::new();
    let mut abs = Vec::new();
    for a in span(up_to / 800.0, up_to, 800) {
        let mie = Mie::new(a, Some(m))?;
        ext.push([a, mie.extinction()]);
        sca.push([a, mie.scattering()]);
        abs.push([a, mie.absorption()]);
    }
    let (a_max, q_max) = ext
        .iter()
        .fold((0.0, 0.0), |b, p| if p[1] > b.1 { (p[0], p[1]) } else { b });
    let top = (1.08 * q_max).max(2.4);
    let ray: Vec<[f64; 2]> = ext
        .iter()
        .map(|p| [p[0], rayleigh(p[0], m)])
        .take_while(|p| p[1] <= top)
        .collect();
    let last = ext.last().map_or(0.0, |p| p[1]);
    let small = Mie::new(0.1, Some(m))?.scattering() / rayleigh(0.1, m) - 1.0;
    let mut series = vec![
        curve("extinction", ext, false, 0),
        curve("scattering", sca, false, 1),
    ];
    if kappa > 0.0 {
        series.push(curve("absorption", abs, false, 2));
    }
    series.push(curve("Rayleigh", ray, true, DIPOLE));
    series.push(curve("Q = 2", vec![[0.0, 2.0], [up_to, 2.0]], true, GREY));
    let figures = vec![
        figure(
            "largest $Q_\\text{ext}$",
            q_max,
            4,
            "",
            &format!("at $\\alpha = {}$", sig(a_max, 4)),
        ),
        figure(
            "$Q_\\text{ext}$ at the end",
            last,
            4,
            "",
            &format!(
                "at $\\alpha = {}$; a large sphere's limit is 2",
                sig(up_to, 3)
            ),
        ),
        figure(
            "Mie over Rayleigh at $\\alpha = 0.1$",
            small,
            2,
            "",
            "less one: the next term is of order $\\alpha^2$",
        ),
    ];
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "size parameter α = 2πn₀ρ/λ".to_owned(),
        x_length: false,
        y_label: "efficiency Q, over πρ²".to_owned(),
        y_log: false,
        y_range: Some([0.0, top]),
        series,
        marker: Some(a_max),
        figures,
        note: format!(
            "Exact, no grid: Mie's series for a sphere of relative index {index} + {kappa}i, summed until a term adds less than 1e-17, at 800 sizes. The line marks the largest extinction."
        ),
    })
}

/// The quasi-static dipole's efficiencies of a sphere of permittivity `eps` in a medium of
/// permittivity `eps_d` at size parameter `x`: (extinction, scattering, absorption), with the
/// polarizability's factor F = (ε − ε_d)/(ε + 2ε_d), Q_abs = 4x Im F and Q_sca = (8/3)x⁴|F|².
pub fn dipole(eps: c64, eps_d: f64, x: f64) -> (f64, f64, f64) {
    let f = (eps - eps_d) / (eps + 2.0 * eps_d);
    let abs = 4.0 * x * f.im;
    let sca = 8.0 / 3.0 * x.powi(4) * f.norm_sqr();
    (abs + sca, sca, abs)
}

/// The peak of `points` and its full width at half its height, the half-height crossings
/// found on the points either side and interpolated.
fn peak(points: &[[f64; 2]]) -> (f64, f64, f64) {
    let (i, top) =
        points.iter().enumerate().fold(
            (0, f64::MIN),
            |a, (i, p)| if p[1] > a.1 { (i, p[1]) } else { a },
        );
    let half = 0.5 * top;
    let cross = |j: usize, k: usize| {
        let ([x0, y0], [x1, y1]) = (points[j], points[k]);
        x0 + (half - y0) * (x1 - x0) / (y1 - y0)
    };
    let left = (1..=i)
        .rev()
        .find(|&j| points[j - 1][1] < half)
        .map(|j| cross(j - 1, j));
    let right = (i..points.len() - 1)
        .find(|&j| points[j + 1][1] < half)
        .map(|j| cross(j, j + 1));
    let width = match (left, right) {
        (Some(l), Some(r)) => r - l,
        _ => f64::NAN,
    };
    (points[i][0], top, width)
}

/// A Drude sphere's efficiencies against ω/ω_p, by Mie's series and the quasi-static dipole.
pub fn sphere(radius: f64, damping: f64, medium: f64) -> photonoxide::Result<ChartData> {
    let metal = drude(1.0, damping)?;
    let eps_d = medium * medium;
    let mut ext = Vec::new();
    let mut sca = Vec::new();
    let mut abs = Vec::new();
    let mut quasi = Vec::new();
    for w in span(0.2, 1.0, 1601) {
        let e = eps(&metal, w)?;
        let s = Sphere {
            radius,
            permittivity: Some(e),
            background: eps_d,
        };
        let c = s.cross_sections(Frequency::natural(w)?)?;
        let shadow = PI * radius * radius;
        ext.push([w, c.extinction / shadow]);
        sca.push([w, c.scattering / shadow]);
        abs.push([w, c.absorption / shadow]);
        let x = 2.0 * PI * medium * radius * w;
        quasi.push([w, dipole(e, eps_d, x).0]);
    }
    let (w_peak, q_peak, width) = peak(&ext);
    let (w_quasi, _, width_quasi) = peak(&quasi);
    let resonance = 1.0 / (1.0 + 2.0 * eps_d).sqrt();
    let at_peak = ext.iter().position(|p| p[0] == w_peak).unwrap_or(0);
    let share = sca[at_peak][1] / ext[at_peak][1];
    let size = 2.0 * PI * medium * radius * w_peak;
    let top = 1.08
        * ext
            .iter()
            .chain(&quasi)
            .map(|p| p[1])
            .fold(0.0, f64::max)
            .min(3.0 * q_peak);
    let figures = vec![
        figure(
            "extinction's peak",
            w_peak,
            4,
            "ω_p",
            &format!(
                "$Q_\\text{{ext}} = {}$ there, $\\alpha = {}$",
                sig(q_peak, 4),
                sig(size, 3)
            ),
        ),
        figure("its full width", width, 3, "ω_p", "at half its height"),
        figure(
            "the dipole's peak",
            w_quasi,
            4,
            "ω_p",
            &format!("quasi-static, full width {} ω_p", sig(width_quasi, 3)),
        ),
        figure(
            "small sphere's resonance",
            resonance,
            4,
            "ω_p",
            "$\\omega_p/\\sqrt{1 + 2\\varepsilon_d}$, where Re $\\varepsilon = -2\\varepsilon_d$ without loss",
        ),
        figure(
            "scattered share at the peak",
            share,
            3,
            "",
            "$Q_\\text{sca}/Q_\\text{ext}$: the rest is absorbed",
        ),
    ];
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "frequency ω/ω_p".to_owned(),
        x_length: false,
        y_label: "efficiency Q, over πρ²".to_owned(),
        y_log: false,
        y_range: Some([0.0, top]),
        series: vec![
            curve("extinction", ext, false, 0),
            curve("scattering", sca, false, 1),
            curve("absorption", abs, false, 2),
            curve("quasi-static dipole", quasi, true, DIPOLE),
        ],
        marker: Some(resonance),
        figures,
        note: format!(
            "Exact, no grid: Mie's series for a sphere of radius {radius} λ_p, ε = 1 − ω_p²/(ω² + iγω) with γ = {damping} ω_p, in a medium of index {medium}, at 1601 frequencies. The dashed curve is the quasi-static dipole, computed in the studio; the line marks where Re ε = −2ε_d without loss."
        ),
    })
}

/// Mie's "absorption of colloidal gold solutions" (his table on p. 438, drawn as his Fig. 25):
/// for spheres 0 (the finest), 20, 40 … 180 nm across, the thousandths of a beam lost over 1 mm
/// of a solution of 1 mm³ of gold per litre, at 420, 450, 500, 525, 550, 600 and 650 nm. He read
/// them off curves through his Tables I to III, which keep a₁, a₂ and p₁ (his §27).
pub const MIE_TABLE: [(f64, [f64; 7]); 10] = [
    (0.0, [40.5, 37.4, 59.6, 69.0, 55.5, 16.4, 7.0]),
    (20.0, [42.7, 38.8, 62.2, 76.6, 61.6, 18.5, 8.0]),
    (40.0, [46.5, 42.0, 66.0, 86.5, 77.7, 25.0, 11.3]),
    (60.0, [48.8, 45.4, 67.4, 88.4, 96.1, 37.3, 16.4]),
    (80.0, [46.5, 44.8, 62.5, 81.0, 97.0, 65.0, 27.1]),
    (100.0, [41.0, 41.0, 52.2, 66.3, 82.2, 85.0, 44.5]),
    (120.0, [34.6, 34.8, 42.8, 51.2, 61.0, 77.5, 58.7]),
    (140.0, [29.4, 29.7, 35.9, 41.9, 48.6, 57.0, 56.1]),
    (160.0, [25.7, 26.1, 30.8, 36.0, 41.2, 45.1, 45.6]),
    (180.0, [24.0, 24.8, 28.0, 33.0, 37.0, 37.7, 36.5]),
];

/// The diameter the series stands in for Mie's finest particles with, nm: small enough that
/// the efficiency over the size no longer changes in the digits he printed.
pub const FINEST: f64 = 1.0;

/// The largest relative difference of the series' extinction from Mie's row at `diameter`
/// (0 for his finest, computed at [`FINEST`]), and the wavelength where it falls, nm.
pub fn against_mie(row: &(f64, [f64; 7])) -> photonoxide::Result<(f64, f64)> {
    let ours = gold(row.0.max(FINEST))?;
    Ok(ours
        .iter()
        .zip(row.1)
        .map(|(r, m)| ((r[1] - m) / m, r[0]))
        .fold((0.0, 0.0), |a, b| if b.0.abs() > a.0.abs() { b } else { a }))
}

/// What 1 mm of a solution of 1 mm³ of gold per litre takes from a beam, in thousandths, for
/// efficiency `q` of spheres `diameter` nm across: the concentration 10⁻⁶ over a sphere's volume
/// is the number of spheres per mm³, each taking q πρ².
pub fn per_mm(q: f64, diameter: f64) -> f64 {
    let rho = 0.5 * diameter * 1e-6;
    1e3 * 1e-6 * q * PI * rho * rho / (4.0 / 3.0 * PI * rho.powi(3))
}

/// Mie's gold spheres `diameter` nm across: (vacuum wavelength nm, extinction, absorption,
/// scattering), each per mm in thousandths.
pub fn gold(diameter: f64) -> photonoxide::Result<Vec<[f64; 4]>> {
    Mie::gold_in_water()
        .iter()
        .map(|g| {
            let size = PI * diameter / g.in_water;
            let mie = Mie::new(size, Some(refractive_index(g.permittivity)))?;
            Ok([
                g.wavelength,
                per_mm(mie.extinction(), diameter),
                per_mm(mie.absorption(), diameter),
                per_mm(mie.scattering(), diameter),
            ])
        })
        .collect()
}

/// A free-electron metal (ε∞ = 1) through gold's permittivity at 650 nm, as Mie took it:
/// (plasma frequency f_p, collision rate γ/2π), both in c/µm. With z = 1 − ε at ω,
/// (ω² + iγω) z = ω_p² is real: γ = −ω Im z/Re z and ω_p² = ω² Re z − γω Im z.
pub fn gold_drude() -> (f64, f64) {
    let g = Mie::gold_in_water()[6];
    let f = 1e3 / g.wavelength;
    let z = 1.0 - g.index * g.index;
    let gamma = -f * z.im / z.re;
    let plasma = (f * f * z.re - gamma * f * z.im).sqrt();
    (plasma, gamma)
}

/// Mie's colloidal gold at his seven wavelengths.
pub fn gold_chart(diameter: f64) -> photonoxide::Result<ChartData> {
    let rows = gold(diameter)?;
    let col = |j: usize| rows.iter().map(|r| [r[0], r[j]]).collect::<Vec<_>>();
    // Mie's row nearest the diameter
    let row = MIE_TABLE
        .iter()
        .min_by(|a, b| (a.0 - diameter).abs().total_cmp(&(b.0 - diameter).abs()))
        .copied()
        .unwrap_or(MIE_TABLE[0]);
    let (worst, worst_at) = against_mie(&row)?;
    let (plasma, gamma) = gold_drude();
    let fitted = Material::new(
        "Drude metal through Mie's gold at 650 nm",
        Model::Drude {
            eps_inf: 1.0,
            plasma: Frequency::natural(plasma)?,
            damping: gamma,
        },
        Wavelength::um(0.01)?,
        Wavelength::um(100.0)?,
        Provenance {
            reference: "fitted to G. Mie, Ann. Phys. 330, 377 (1908), p. 417, at 650 nm".to_owned(),
            doi: "10.1002/andp.19083300302".to_owned(),
            data: String::new(),
            temperature: None,
            notes: String::new(),
        },
    )?;
    // the free-electron metal is a formula: drawn every 5 nm, water's index λ/λ′ interpolated
    // between Mie's wavelengths
    let table = Mie::gold_in_water();
    let water_at = |nm: f64| {
        let k = table
            .windows(2)
            .position(|w| nm <= w[1].wavelength)
            .unwrap_or(table.len() - 2);
        let (a, b) = (table[k], table[k + 1]);
        let (na, nb) = (a.wavelength / a.in_water, b.wavelength / b.in_water);
        na + (nb - na) * (nm - a.wavelength) / (b.wavelength - a.wavelength)
    };
    let mut drude_ext = Vec::new();
    for nm in span(420.0, 650.0, 47) {
        let water = water_at(nm);
        let e = fitted.permittivity(Wavelength::nm(nm)?)? / (water * water);
        let mie = Mie::new(PI * diameter * water / nm, Some(refractive_index(e)))?;
        drude_ext.push([nm, per_mm(mie.extinction(), diameter)]);
    }
    // the fitted metal's dipole resonance in water: Re ε = −2 n_w², water's index from 650 nm
    let g650 = Mie::gold_in_water()[6];
    let water = g650.wavelength / g650.in_water;
    let resonance = bisect(150.0, 650.0, |nm| {
        Ok(fitted.permittivity(Wavelength::nm(nm)?)?.re + 2.0 * water * water)
    })?;
    let (l_peak, k_peak) = rows
        .iter()
        .fold((0.0, 0.0), |a, r| if r[1] > a.1 { (r[0], r[1]) } else { a });
    let peak_row = rows
        .iter()
        .find(|r| r[0] == l_peak)
        .copied()
        .unwrap_or([0.0; 4]);
    let figures = vec![
        figure(
            "extinction's peak",
            l_peak,
            3,
            "nm",
            &format!("{} ‰ per mm, of Mie's seven wavelengths", sig(k_peak, 3)),
        ),
        figure(
            "scattered share there",
            peak_row[3] / peak_row[1],
            3,
            "",
            "the rest is absorbed",
        ),
        figure(
            "against Mie's table",
            worst,
            2,
            "",
            &format!(
                "the largest relative difference from his row for {} nm (p. 438), at {} nm",
                row.0, worst_at
            ),
        ),
        figure(
            "Drude fit's resonance",
            resonance,
            3,
            "nm",
            &format!(
                "in water, the free-electron metal through gold at 650 nm: $\\lambda_p$ = {} nm, $\\gamma/\\omega_p$ = {}",
                sig(1e3 / plasma, 3),
                sig(gamma / plasma, 3)
            ),
        ),
    ];
    let mie_dots: Vec<[f64; 2]> = rows.iter().zip(row.1).map(|(r, m)| [r[0], m]).collect();
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "vacuum wavelength λ, nm".to_owned(),
        x_length: false,
        y_label: "lost over 1 mm, ‰".to_owned(),
        y_log: false,
        y_range: None,
        series: vec![
            curve("extinction", col(1), false, 0),
            curve("absorption", col(2), false, 2),
            curve("scattering", col(3), false, 1),
            curve(
                &if row.0 == 0.0 {
                    "Mie 1908, finest".to_owned()
                } else {
                    format!("Mie 1908, {} nm", row.0)
                },
                mie_dots,
                true,
                GREY,
            ),
            curve("Drude fit, extinction", drude_ext, true, 5),
        ],
        marker: Some(l_peak),
        figures,
        note: format!(
            "Exact, no grid: Mie's series for gold spheres {diameter} nm across in water, gold's m′² from Mie's table on p. 417, at his seven wavelengths, the points joined by straight lines; 1 mm³ of gold per litre."
        ),
    })
}

/// Every chart of the lesson, by id, at its parameters.
pub fn evaluate(id: &str, at: impl Fn(&str) -> f64) -> Option<photonoxide::Result<ChartData>> {
    Some(match id {
        "plasmon-drude" => drude_chart(at("damping"), at("background")),
        "plasmon-dispersion" => dispersion(at("medium"), at("damping")),
        "plasmon-length" => length(at("medium"), at("damping"), at("at")),
        "mie-efficiency" => efficiency(at("index"), at("kappa"), at("up_to")),
        "mie-plasmon" => sphere(at("radius"), at("damping"), at("medium")),
        "mie-gold" => gold_chart(at("diameter")),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "{a} against {b}");
    }

    fn fig(data: &ChartData, label: &str) -> f64 {
        data.figures
            .iter()
            .find(|f| f.label.starts_with(label))
            .unwrap_or_else(|| panic!("no figure {label}"))
            .value
            .unwrap()
    }

    fn series<'a>(data: &'a ChartData, label: &str) -> &'a [[f64; 2]] {
        &data
            .series
            .iter()
            .find(|c| c.label == label)
            .unwrap_or_else(|| panic!("no curve {label}"))
            .points
    }

    /// The chart's permittivity is Drude's, ε∞ − 1/(w² + iγw) at w = ω/ω_p, and it crosses
    /// zero at √(1/ε∞ − γ²).
    #[test]
    fn the_drude_metal_is_drudes() {
        for (d, b) in [(0.02, 1.0), (0.1, 4.0), (0.3, 9.0)] {
            let data = drude_chart(d, b).unwrap();
            for (p, q) in series(&data, "Re ε")
                .iter()
                .zip(series(&data, "Im ε"))
                .step_by(17)
            {
                let w = p[0];
                let e = b - 1.0 / c64::new(w * w, d * w);
                close(p[1], e.re, 1e-12 * e.norm());
                close(q[1], e.im, 1e-12 * e.norm());
            }
            close(
                fig(&data, "Re $\\varepsilon = 0$"),
                (1.0 / b - d * d).sqrt(),
                1e-9,
            );
        }
        // without much loss a metal below ω_p reflects nearly all; above, little
        let data = drude_chart(0.002, 1.0).unwrap();
        assert!(fig(&data, "reflectance at $0.5") > 0.99);
        assert!(fig(&data, "reflectance at $1.5") < 0.1);
    }

    /// The stack's TM mode is the closed form √(ε_m ε_d/(ε_m + ε_d)) to round-off, at every
    /// frequency the plasmon is bound, beyond the light line, and approaches ω_sp.
    #[test]
    fn the_surface_plasmon_is_the_closed_form() {
        for (n, d) in [(1.0, 0.01), (1.5, 0.05), (2.0, 0.002)] {
            let data = dispersion(n, d).unwrap();
            assert!(fig(&data, "against the closed form") < 1e-9);
            let sp = surface_frequency(n);
            let curve = series(&data, "surface plasmon (TM)");
            assert!(curve.len() > 300);
            for p in curve {
                assert!(p[0] > n * p[1], "inside the light line at {p:?}");
            }
            // the curve reaches ω_sp, give or take the loss
            let w_top = curve.iter().map(|p| p[1]).fold(0.0, f64::max);
            assert!((w_top - sp).abs() < 0.1 * sp, "{w_top} against {sp}");
        }
    }

    /// No TE wave is bound to the interface: the TE secant iteration started on the closed form
    /// doesn't land on a mode beyond the light line with a field decaying both ways.
    #[test]
    fn no_te_wave_is_bound() {
        let metal = drude(1.0, 0.01).unwrap();
        for w in [0.2, 0.4, 0.6] {
            let e = eps(&metal, w).unwrap();
            let stack = interface(1.0, e).unwrap();
            let found = stack.mode_near(
                Polarization::Te,
                Wavelength::um(1.0 / w).unwrap(),
                closed_form(e, 1.0),
            );
            if let Ok(mode) = found {
                let b = mode.effective_index();
                assert!(b.re <= 1.0 + 1e-9, "a TE mode at {b}");
            }
        }
    }

    /// Far below ω_sp the plasmon hugs the light line and reaches far into the dielectric; near
    /// it, it is held tightly and travels a short way. The lengths are the closed form's.
    #[test]
    fn confinement_costs_propagation() {
        let low = length(1.0, 0.01, 0.2).unwrap();
        let high = length(1.0, 0.01, 0.9).unwrap();
        assert!(fig(&low, "propagation length") > 10.0 * fig(&high, "propagation length"));
        assert!(
            fig(&low, "depth in the dielectric") > 10.0 * fig(&high, "depth in the dielectric")
        );
        // the closed form's lengths at the same frequency
        let w = 0.2 * surface_frequency(1.0);
        let e = eps(&drude(1.0, 0.01).unwrap(), w).unwrap();
        let k = closed_form(e, 1.0);
        let k0 = 2.0 * PI * w;
        close(
            fig(&low, "propagation length"),
            1.0 / (2.0 * k.im * k0),
            1e-8 * fig(&low, "propagation length"),
        );
        let kd = refractive_index(1.0 - k * k).im * k0;
        close(fig(&low, "depth in the dielectric"), 1.0 / kd, 1e-8 / kd);
        // ε_m ≫ ε_d: the depth in the dielectric over the metal's is about |ε_m|/ε_d
        let ratio = fig(&low, "depth in the dielectric") / fig(&low, "depth in the metal");
        close(ratio / e.norm(), 1.0, 0.05);
    }

    /// Mie's efficiencies: a lossless sphere scatters what it takes, Rayleigh's limit at small
    /// α, and extinction towards 2 at large α.
    #[test]
    fn mies_efficiencies() {
        let data = efficiency(1.5, 0.0, 60.0).unwrap();
        for (e, s) in series(&data, "extinction")
            .iter()
            .zip(series(&data, "scattering"))
        {
            close(e[1], s[1], 1e-9 * e[1].max(1e-12));
        }
        assert!(fig(&data, "Mie over Rayleigh").abs() < 0.02);
        close(fig(&data, "$Q_\\text{ext}$ at the end"), 2.0, 0.25);
        // the first maximum of a glass sphere: Q_ext above 3, near α(m − 1) = 2
        let q = fig(&data, "largest $Q_\\text{ext}$");
        assert!(q > 3.0 && q < 4.5, "{q}");
        // a lossy sphere absorbs
        let lossy = efficiency(1.5, 0.5, 20.0).unwrap();
        assert!(series(&lossy, "absorption").iter().all(|p| p[1] >= 0.0));
    }

    /// A small Drude sphere resonates where Re ε = −2ε_d, as the quasi-static dipole does, and
    /// its resonance moves to lower frequencies and widens as it grows.
    #[test]
    fn the_localized_plasmon() {
        for n in [1.0, 1.33] {
            let small = sphere(0.002, 0.03, n).unwrap();
            let w = fig(&small, "extinction's peak");
            close(w, fig(&small, "small sphere"), 0.01);
            close(w, fig(&small, "the dipole's peak"), 2e-3);
        }
        let mut last = (f64::MAX, 0.0);
        for r in [0.005, 0.03, 0.06, 0.1] {
            let data = sphere(r, 0.03, 1.0).unwrap();
            let w = fig(&data, "extinction's peak");
            let width = fig(&data, "its full width");
            assert!(w < last.0, "{r}: {w} not below {}", last.0);
            assert!(width > last.1, "{r}: {width} not above {}", last.1);
            last = (w, width);
        }
        // a small sphere absorbs; a large one scatters
        assert!(fig(&sphere(0.005, 0.03, 1.0).unwrap(), "scattered share") < 0.1);
        assert!(fig(&sphere(0.2, 0.03, 1.0).unwrap(), "scattered share") > 0.5);
    }

    /// The quasi-static efficiencies are Mie's first term for a small sphere.
    #[test]
    fn the_dipole_is_mies_limit() {
        let eps_s = c64::new(-2.5, 0.4);
        let x = 0.01;
        let mie = Mie::new(x, Some(refractive_index(eps_s))).unwrap();
        let (ext, sca, abs) = dipole(eps_s, 1.0, x);
        close(mie.extinction() / ext, 1.0, 1e-3);
        close(mie.absorption() / abs, 1.0, 1e-3);
        close(mie.scattering() / sca, 1.0, 1e-3);
    }

    /// Mie's gold solutions: the series reproduces his finest row of p. 438 to 1.2 %, the half
    /// unit of his last digit and his two-decimal index, and every row to within 17 %, the most
    /// at 650 nm on spheres 40 nm across, where his numbers, read off curves through his hand
    /// computations, stand above the series; the median over his 70 numbers is within 3 %.
    /// Each row's strongest wavelength is his but for 180 nm, where his 600 and 550 nm differ by
    /// 2 %.
    #[test]
    fn mies_gold_solutions() {
        let data = gold_chart(1.0).unwrap();
        assert!(fig(&data, "against Mie's table").abs() < 0.013);
        close(fig(&data, "extinction's peak"), 525.0, 0.0);
        let mut all = Vec::new();
        for row in &MIE_TABLE {
            let (worst, _) = against_mie(row).unwrap();
            assert!(worst.abs() < 0.17, "{}: {worst}", row.0);
            let ours = gold(row.0.max(FINEST)).unwrap();
            for (r, m) in ours.iter().zip(row.1) {
                all.push(((r[1] - m) / m).abs());
            }
            let strongest = |v: Vec<f64>| {
                v.iter()
                    .enumerate()
                    .fold((0, 0.0), |a, (i, &x)| if x > a.1 { (i, x) } else { a })
                    .0
            };
            if row.0 < 170.0 {
                assert_eq!(
                    strongest(ours.iter().map(|r| r[1]).collect()),
                    strongest(row.1.to_vec()),
                    "{} nm",
                    row.0
                );
            }
        }
        all.sort_by(f64::total_cmp);
        assert!(all[all.len() / 2] < 0.03, "median {}", all[all.len() / 2]);
        // larger spheres scatter more, and their extinction peaks further to the red
        let big = gold_chart(140.0).unwrap();
        assert!(fig(&big, "scattered share") > fig(&data, "scattered share"));
        assert!(fig(&big, "extinction's peak") > 525.0);
        // a free-electron metal through gold at 650 nm resonates far to the blue of gold's
        assert!(fig(&data, "Drude fit's resonance") < 420.0);
    }
}
