//! The material dispersion lesson's charts: a bound charge's resonance as a Lorentz medium
//! (`material::Model::Lorentz`), a Cauchy formula fitted to Malitson's silica against the formula
//! itself (`Model::Cauchy`, `Model::Sellmeier`), the catalogue's indices with their group
//! indices (`Material::refractive_index`, `Material::group_index`), and the group-velocity
//! dispersion D of silica and silicon from their indices (`mode::dispersion::dispersion`).
//! Nothing here has a grid: every curve is a model evaluated at its wavelengths.

use num_complex::Complex64 as c64;
use photonoxide::material::catalogue::{self, Conditions};
use photonoxide::material::{self, LorentzPole, Material, Model, Provenance};
use photonoxide::mode::dispersion;
use photonoxide::units::{Frequency, Wavelength};

use super::{ChartData, Curve, Figure, figure, sig};

/// The palette's grey, for reference lines.
const GREY: usize = 7;

fn wavelength(um: f64) -> photonoxide::Result<Wavelength> {
    Wavelength::um(um)
}

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
    // the last point is `b` itself, not a rounding of it past a model's range
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

fn provenance(reference: &str) -> Provenance {
    Provenance {
        reference: reference.to_owned(),
        doi: String::new(),
        data: String::new(),
        temperature: None,
        notes: String::new(),
    }
}

/// A single Lorentz oscillator in a background ε∞, resonant at a vacuum wavelength of 1 µm, so
/// that a frequency in units of the resonance's, ω/ω₀, is 1/λ with λ in µm.
pub fn oscillator(strength: f64, damping: f64, background: f64) -> photonoxide::Result<Material> {
    Material::new(
        "Lorentz oscillator",
        Model::Lorentz {
            eps_inf: background,
            poles: vec![LorentzPole {
                strength,
                resonance: Frequency::natural(1.0)?,
                // γ/2π in c/µm: with f₀ = 1 c/µm, γ/ω₀ itself
                damping,
            }],
        },
        wavelength(1.0 / 4.0)?,
        wavelength(1.0 / 0.02)?,
        provenance("ε(ω) = ε∞ + Δε ω₀² / (ω₀² − ω² − iγω)"),
    )
}

/// The complex index of a Lorentz oscillator at ω/ω₀ = `w`.
fn oscillator_index(m: &Material, w: f64) -> photonoxide::Result<c64> {
    m.refractive_index(wavelength(1.0 / w)?)
}

/// The index n and extinction κ of one bound resonance against frequency.
pub fn oscillator_chart(
    strength: f64,
    damping: f64,
    background: f64,
) -> photonoxide::Result<ChartData> {
    let m = oscillator(strength, damping, background)?;
    let (lo, hi) = (0.05, 2.5);
    // evenly across, and finely across the line, ten widths either side of the resonance
    let mut ws = span(lo, hi, 600);
    ws.extend(span(
        (1.0 - 10.0 * damping).max(lo),
        1.0 + 10.0 * damping,
        801,
    ));
    ws.sort_by(f64::total_cmp);
    let mut n = Vec::with_capacity(ws.len());
    let mut k = Vec::with_capacity(ws.len());
    for &w in &ws {
        let z = oscillator_index(&m, w)?;
        n.push([w, z.re]);
        k.push([w, z.im]);
    }
    // the anomalous band, where n falls as ω rises: between the extremes of n round ω₀
    let (w_max, n_max) =
        n.iter()
            .filter(|p| p[0] < 1.0 + 3.0 * damping)
            .fold(
                (0.0, f64::MIN),
                |a, p| if p[1] > a.1 { (p[0], p[1]) } else { a },
            );
    let (w_min, n_min) = n
        .iter()
        .filter(|p| p[0] > w_max)
        .fold(
            (0.0, f64::MAX),
            |a, p| if p[1] < a.1 { (p[0], p[1]) } else { a },
        );
    let (w_peak, k_peak) = k
        .iter()
        .fold((0.0, 0.0), |a, p| if p[1] > a.1 { (p[0], p[1]) } else { a });
    let static_n = (background + strength).sqrt();
    let figures = vec![
        figure(
            "$n$ at $\\omega \\to 0$",
            static_n,
            5,
            "",
            "$\\sqrt{\\varepsilon_\\infty + \\Delta\\varepsilon}$",
        ),
        figure(
            "$n$ at $\\omega = 0.5\\thinspace\\omega_0$",
            oscillator_index(&m, 0.5)?.re,
            5,
            "",
            "transparent: $\\kappa$ is small",
        ),
        figure(
            "$\\kappa$'s peak",
            k_peak,
            4,
            "",
            &format!("at $\\omega = {}\\thinspace\\omega_0$", sig(w_peak, 4)),
        ),
        figure(
            "anomalous band, $dn/d\\omega \\lt 0$",
            w_min - w_max,
            3,
            "ω₀",
            &format!("from {} to {} $\\omega_0$", sig(w_max, 4), sig(w_min, 4)),
        ),
        figure("$n$'s highest", n_max, 4, "", "just below $\\omega_0$"),
        figure(
            "$n$'s lowest",
            n_min,
            4,
            "",
            if n_min < 1.0 {
                "below 1, just above $\\omega_0$"
            } else {
                "just above $\\omega_0$"
            },
        ),
    ];
    let top = n_max.max(k_peak) * 1.05;
    let series = vec![
        curve("refractive index n", n, false, 0),
        curve("extinction κ", k, false, 1),
        curve("n = 1", vec![[lo, 1.0], [hi, 1.0]], true, GREY),
    ];
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "frequency ω/ω₀".to_owned(),
        x_length: false,
        y_label: "n and κ".to_owned(),
        y_log: false,
        y_range: Some([0.0, top]),
        series,
        marker: Some(1.0),
        figures,
        note: format!(
            "Exact, no grid: ε(ω) = {background} + {strength} ω₀² / (ω₀² − ω² − iγω) with γ = {damping} ω₀, and n + iκ = √ε. The line marks the resonance, ω = ω₀."
        ),
    })
}

/// Malitson's silica, the catalogue's default model of fused silica.
fn silica() -> Material {
    material::silica()
}

/// The least-squares Cauchy formula n = A₀ + A₁/λ² + A₂/λ⁴ through `points` (λ µm, n): the
/// normal equations of three unknowns, solved by Cramer's rule.
pub fn cauchy_fit(points: &[[f64; 2]]) -> [f64; 3] {
    let mut a = [[0.0; 3]; 3];
    let mut b = [0.0; 3];
    for &[l, n] in points {
        let x = 1.0 / (l * l);
        let basis = [1.0, x, x * x];
        for i in 0..3 {
            for j in 0..3 {
                a[i][j] += basis[i] * basis[j];
            }
            b[i] += basis[i] * n;
        }
    }
    let det = |m: &[[f64; 3]; 3]| {
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    };
    let d = det(&a);
    let mut out = [0.0; 3];
    for (k, o) in out.iter_mut().enumerate() {
        let mut m = a;
        for i in 0..3 {
            m[i][k] = b[i];
        }
        *o = det(&m) / d;
    }
    out
}

/// The shortest wavelength the Cauchy fit starts from, µm.
const CAUCHY_FROM: f64 = 0.4;

/// A Cauchy formula fitted to Malitson's silica from 0.4 µm to `fit_to` µm, against the
/// formula over its whole range.
pub fn cauchy(fit_to: f64) -> photonoxide::Result<ChartData> {
    let sio2 = silica();
    let n = |l: f64| -> photonoxide::Result<f64> { Ok(sio2.refractive_index(wavelength(l)?)?.re) };
    let fitted: Vec<[f64; 2]> = span(CAUCHY_FROM, fit_to, 60)
        .into_iter()
        .map(|l| Ok([l, n(l)?]))
        .collect::<photonoxide::Result<_>>()?;
    let coefficients = cauchy_fit(&fitted);
    let (lo, hi) = sio2.range();
    let fit = Material::new(
        "Cauchy fit",
        Model::Cauchy(coefficients.to_vec()),
        lo,
        hi,
        provenance("a Cauchy formula fitted to Malitson's silica"),
    )?;
    let c = |l: f64| -> photonoxide::Result<f64> { Ok(fit.refractive_index(wavelength(l)?)?.re) };
    let ls = span(lo.to_um(), hi.to_um(), 400);
    let mut malitson = Vec::new();
    let mut cauchy = Vec::new();
    for &l in &ls {
        malitson.push([l, n(l)?]);
        cauchy.push([l, c(l)?]);
    }
    let inside = fitted
        .iter()
        .map(|&[l, v]| Ok((c(l)? - v).abs()))
        .collect::<photonoxide::Result<Vec<f64>>>()?
        .into_iter()
        .fold(0.0, f64::max);
    let error = |l: f64| -> photonoxide::Result<f64> { Ok(c(l)? - n(l)?) };
    // how far from the band's start the fit holds to four decimals throughout
    let holds = ls
        .iter()
        .copied()
        .filter(|&l| l >= CAUCHY_FROM)
        .map(|l| Ok((l, error(l)?.abs())))
        .collect::<photonoxide::Result<Vec<_>>>()?
        .into_iter()
        .take_while(|&(_, e)| e <= 1e-4)
        .last()
        .map_or(CAUCHY_FROM, |(l, _)| l);
    let figures = vec![
        figure(
            "the fit's largest error inside its band",
            inside,
            2,
            "",
            &format!("from {CAUCHY_FROM} to {} µm", sig(fit_to, 3)),
        ),
        figure(
            "within $10^{-4}$ from 0.4 µm up to",
            holds,
            3,
            "µm",
            "four decimals",
        ),
        figure(
            "error at 1.55 µm",
            error(1.55)?,
            2,
            "",
            "Cauchy minus Malitson",
        ),
        figure("error at 3.0 µm", error(3.0)?, 2, "", ""),
        figure(
            "$A_0$",
            coefficients[0],
            6,
            "",
            "$n = A_0 + A_1/\\lambda^2 + A_2/\\lambda^4$",
        ),
        figure("$A_1$", coefficients[1], 4, "µm²", ""),
    ];
    let low = malitson.last().map_or(1.39, |p| p[1]) - 0.01;
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "wavelength λ (µm)".to_owned(),
        x_length: false,
        y_label: "refractive index n".to_owned(),
        y_log: false,
        y_range: Some([low, 1.54]),
        series: vec![
            curve("Malitson's Sellmeier formula", malitson, false, 0),
            curve("Cauchy, fitted to it", cauchy, true, 1),
        ],
        marker: Some(fit_to),
        figures,
        note: format!(
            "Exact, no grid: Malitson's three-term Sellmeier formula for fused silica at 20 °C, and the Cauchy formula n = A₀ + A₁/λ² + A₂/λ⁴ fitted to it by least squares at 60 wavelengths from {CAUCHY_FROM} µm to the marked {} µm.",
            sig(fit_to, 3)
        ),
    })
}

/// A material the index chart shows: its name, its model's name, and the material.
pub struct Shown {
    pub name: &'static str,
    pub model: &'static str,
    pub material: Material,
}

/// The index chart's materials, by number from 1: Malitson's silica, Li's silicon, Luke's
/// nitride, and Leviton and Frey's silica at 295 K.
pub fn shown(which: usize) -> photonoxide::Result<Shown> {
    Ok(match which {
        1 => Shown {
            name: "fused silica",
            model: "Malitson 1965, 20 °C, relative to air",
            material: material::silica(),
        },
        2 => Shown {
            name: "silicon",
            model: "Li 1980, Table 1, 293 K",
            material: material::silicon(),
        },
        3 => Shown {
            name: "silicon nitride",
            model: "Luke et al. 2015, LPCVD",
            material: material::silicon_nitride(),
        },
        _ => {
            let entry = catalogue::entry("sio2")
                .ok_or_else(|| super::invalid("material", "no silica in the catalogue"))?;
            let model = entry
                .index
                .iter()
                .find(|m| m.id == "leviton-frey-2006")
                .ok_or_else(|| super::invalid("material", "no Leviton and Frey model"))?;
            let material = model
                .materials(Conditions {
                    temperature: Some(295.0),
                    composition: None,
                })?
                .remove(0);
            Shown {
                name: "fused silica",
                model: "Leviton & Frey 2006, 295 K, absolute",
                material,
            }
        }
    })
}

/// A material's index and group index across its whole range, and their values at `at` µm.
pub fn index(which: usize, at: f64) -> photonoxide::Result<ChartData> {
    let s = shown(which)?;
    let m = &s.material;
    let (lo, hi) = m.range();
    let (lo, hi) = (lo.to_um(), hi.to_um());
    let ls = span(lo, hi, 400);
    let mut n = Vec::new();
    let mut ng = Vec::new();
    for &l in &ls {
        let w = wavelength(l)?;
        n.push([l, m.refractive_index(w)?.re]);
        ng.push([l, m.group_index(w)?]);
    }
    let p = m.provenance();
    let mut figures = Vec::new();
    let inside = (lo..=hi).contains(&at);
    if inside {
        let w = wavelength(at)?;
        let n_at = m.refractive_index(w)?.re;
        let ng_at = m.group_index(w)?;
        figures.push(figure(
            &format!("$n$ at {} µm", sig(at, 4)),
            n_at,
            6,
            "",
            "",
        ));
        figures.push(figure(
            &format!("$n_g$ at {} µm", sig(at, 4)),
            ng_at,
            5,
            "",
            "$n - \\lambda\\thinspace dn/d\\lambda$",
        ));
        figures.push(figure("$dn/d\\lambda$", (n_at - ng_at) / at, 4, "/µm", ""));
        if which == 4 {
            let malitson = material::silica().refractive_index(w)?.re;
            figures.push(figure(
                "minus Malitson's",
                n_at - malitson,
                3,
                "",
                "his index is relative to air",
            ));
        }
    } else {
        figures.push(Figure {
            label: format!("$n$ at {} µm", sig(at, 4)),
            value: None,
            text: "outside the model".to_owned(),
            unit: String::new(),
            note: "the catalogue refuses it".to_owned(),
        });
    }
    figures.push(Figure {
        label: "valid".to_owned(),
        value: None,
        text: format!("{} to {} µm", sig(lo, 3), sig(hi, 3)),
        unit: String::new(),
        note: p
            .temperature
            .map(|t| format!("at {} K", sig(t, 4)))
            .unwrap_or_else(|| "no temperature stated".to_owned()),
    });
    figures.push(Figure {
        label: "source".to_owned(),
        value: None,
        text: p.reference.clone(),
        unit: String::new(),
        note: if p.doi.is_empty() {
            String::new()
        } else {
            format!("doi:{}", p.doi)
        },
    });
    let all = n.iter().chain(&ng).map(|q| q[1]);
    let (min, max) = all.fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(v), b.max(v)));
    let pad = 0.05 * (max - min);
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "wavelength λ (µm)".to_owned(),
        x_length: false,
        y_label: "index".to_owned(),
        y_log: false,
        y_range: Some([min - pad, max + pad]),
        series: vec![
            curve("refractive index n", n, false, 0),
            curve("group index n_g", ng, true, 0),
        ],
        marker: inside.then_some(at),
        figures,
        note: format!(
            "Exact, no grid: {}, {}, over the whole range it is valid for; n_g from the model's own derivative.",
            s.name, s.model
        ),
    })
}

/// The wavelengths the dispersion chart samples, µm: from 1.2 (silicon's shortest) to 2.5.
const GVD_FROM: f64 = 1.2;
const GVD_TO: f64 = 2.5;

/// D = −(λ/c) d²n/dλ² of a material from its index sampled every `step` µm, ps/(nm km).
pub fn gvd_of(m: &Material, ls: &[f64]) -> photonoxide::Result<Vec<f64>> {
    let ws: Vec<Wavelength> = ls
        .iter()
        .map(|&l| wavelength(l))
        .collect::<photonoxide::Result<_>>()?;
    let n: Vec<f64> = ws
        .iter()
        .map(|&w| Ok(m.refractive_index(w)?.re))
        .collect::<photonoxide::Result<_>>()?;
    dispersion::dispersion(&ws, &n)
}

/// The zero of `d` (ps/(nm km)) along `ls`, by linear interpolation between the samples that
/// straddle it, if it has one.
fn zero(ls: &[f64], d: &[f64]) -> Option<f64> {
    (1..ls.len())
        .find(|&k| d[k - 1].signum() != d[k].signum())
        .map(|k| ls[k - 1] + (ls[k] - ls[k - 1]) * d[k - 1] / (d[k - 1] - d[k]))
}

/// The temperature at which the dispersion chart takes Frey et al.'s silicon, K: their room
/// temperature, the warmest they measured near it.
pub const FREY_T: f64 = 295.0;

/// Silicon from Frey, Leviton and Madison's temperature-dependent Sellmeier formula (the
/// catalogue's `frey-2006`) at `FREY_T`: smooth, so its second derivative is too.
pub fn silicon_frey() -> photonoxide::Result<Material> {
    let entry = catalogue::entry("si")
        .ok_or_else(|| super::invalid("material", "no silicon in the catalogue"))?;
    let model = entry
        .index
        .iter()
        .find(|m| m.id == "frey-2006")
        .ok_or_else(|| super::invalid("material", "no Frey et al. model"))?;
    Ok(model
        .materials(Conditions {
            temperature: Some(FREY_T),
            composition: None,
        })?
        .remove(0))
}

/// The dispersion chart's curve labels for silicon: Frey et al.'s formula and Li's table.
pub const SI_FREY: &str = "silicon, Frey et al. 2006 formula, 295 K (valid 1.1–5.6 µm, 20–300 K)";
pub const SI_LI: &str = "silicon, Li 1980 table splined, 293 K (valid 1.2–14 µm)";

/// The group-velocity dispersion of silica (`which` 1) or silicon (2, Frey et al.'s smooth
/// formula with Li's table beside it) against wavelength, and each at `at` µm.
pub fn gvd(which: usize, at: f64) -> photonoxide::Result<ChartData> {
    // 2 nm samples: three-point differences of a smooth formula, exact on a parabola
    let ls = span(GVD_FROM, GVD_TO, 651);
    let sio2 = silica();
    let si = material::silicon();
    let frey = silicon_frey()?;
    let d_sio2 = gvd_of(&sio2, &ls)?;
    let d_si = gvd_of(&si, &ls)?;
    let d_frey = gvd_of(&frey, &ls)?;
    let pick = |d: &[f64]| -> Vec<[f64; 2]> {
        // the ends' one-sided differences are left out
        ls.iter()
            .zip(d)
            .skip(1)
            .take(ls.len() - 2)
            .map(|(&l, &v)| [l, v])
            .collect()
    };
    let at_d = |m: &Material| -> photonoxide::Result<f64> {
        let l = [at - 0.002, at, at + 0.002];
        Ok(gvd_of(m, &l)?[1])
    };
    // silicon from Li's rounded table: its D between two knots, where the spline is a cubic,
    // against D one knot either side
    let knots = [1.50, 1.55, 1.60];
    let rounding = knots
        .iter()
        .map(|&l| at_d_at(&si, l))
        .collect::<photonoxide::Result<Vec<f64>>>()?;
    let zero_sio2 = zero(&ls, &d_sio2);
    let figures = vec![
        figure(
            &format!("silica's $D$ at {} µm", sig(at, 4)),
            at_d(&sio2)?,
            4,
            "ps/(nm km)",
            "Malitson's formula; positive: anomalous",
        ),
        figure(
            &format!("silicon's $D$ at {} µm", sig(at, 4)),
            at_d(&frey)?,
            4,
            "ps/(nm km)",
            "Frey et al.'s formula, 295 K; negative: normal",
        ),
        figure(
            &format!("from Li's table at {} µm", sig(at, 4)),
            at_d(&si)?,
            3,
            "ps/(nm km)",
            "splined, 293 K",
        ),
        match zero_sio2 {
            Some(l) => figure("silica's zero dispersion", l, 5, "µm", "$D = 0$"),
            None => Figure {
                label: "silica's zero dispersion".to_owned(),
                value: None,
                text: "none in the window".to_owned(),
                unit: String::new(),
                note: String::new(),
            },
        },
        figure(
            "Li's table's $D$ at 1.50, 1.55, 1.60 µm",
            rounding[1],
            3,
            "ps/(nm km)",
            &format!(
                "{}, {} and {}: the table's rounding",
                sig(rounding[0], 3),
                sig(rounding[1], 3),
                sig(rounding[2], 3)
            ),
        ),
    ];
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "wavelength λ (µm)".to_owned(),
        x_length: false,
        y_label: "dispersion D (ps/(nm km))".to_owned(),
        y_log: false,
        y_range: None,
        // one material at a time: silica's D is tens of ps/(nm km), silicon's hundreds
        series: if which == 1 {
            vec![
                curve(
                    "fused silica, Malitson 1965 formula, 20 °C (valid 0.21–3.71 µm)",
                    pick(&d_sio2),
                    false,
                    0,
                ),
                curve("D = 0", vec![[GVD_FROM, 0.0], [GVD_TO, 0.0]], true, GREY),
            ]
        } else {
            vec![
                curve(SI_FREY, pick(&d_frey), false, 1),
                curve(SI_LI, pick(&d_si), true, 2),
                curve("D = 0", vec![[GVD_FROM, 0.0], [GVD_TO, 0.0]], true, GREY),
            ]
        },
        marker: Some(at),
        figures,
        note: format!(
            "No grid: D = −(λ/c) d²n/dλ² from three-point differences of each material's index at samples 2 nm apart, from {GVD_FROM} to {GVD_TO} µm. Silica's index is Malitson's smooth formula (doi:10.1364/JOSA.55.001205). Silicon's solid curve is Frey, Leviton and Madison's temperature-dependent Sellmeier formula at {FREY_T} K, the absolute index, smooth (doi:10.1117/12.672850); the dashed one is a natural cubic spline through Li's table at 293 K, rounded to four decimals, whose second derivative is a straight line between knots, so its D zig-zags (doi:10.1063/1.555624)."
        ),
    })
}

/// D at `l` µm from differences 2 nm either side.
fn at_d_at(m: &Material, l: f64) -> photonoxide::Result<f64> {
    Ok(gvd_of(m, &[l - 0.002, l, l + 0.002])?[1])
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

    /// The oscillator's index is the driven, damped charge's: ε = ε∞ + Δε/(1 − w² − iγw) at
    /// w = ω/ω₀, n + iκ its square root with κ ≥ 0.
    #[test]
    fn the_oscillator_is_the_driven_charges() {
        for (s, g, b) in [(1.0, 0.05, 1.0), (3.0, 0.2, 2.25), (0.5, 0.01, 1.0)] {
            let data = oscillator_chart(s, g, b).unwrap();
            let n = series(&data, "refractive index n");
            let k = series(&data, "extinction κ");
            for (p, q) in n.iter().zip(k).step_by(13) {
                let w = p[0];
                let eps = b + s / c64::new(1.0 - w * w, -g * w);
                let z = eps.sqrt();
                close(p[1], z.re, 1e-12);
                close(q[1], z.im, 1e-12);
                assert!(q[1] >= 0.0);
            }
            close(fig(&data, "$n$ at $\\omega \\to 0$"), (b + s).sqrt(), 1e-12);
        }
    }

    /// The anomalous band is about γ wide when the damping is light: for a weak oscillator
    /// (n ≈ 1 + χ/2) the extremes of Re χ = Δε(1 − w²)/((1 − w²)² + γ²w²) lie near w = 1 ∓ γ/2.
    #[test]
    fn the_anomalous_band_is_gamma_wide() {
        for g in [0.02, 0.05, 0.1] {
            let data = oscillator_chart(0.02, g, 1.0).unwrap();
            let band = fig(&data, "anomalous band");
            close(band, g, 0.2 * g);
        }
    }

    /// A Lorentz pole with no damping is a Sellmeier term: Malitson's three terms as three
    /// undamped oscillators at 1/C_j give his formula.
    #[test]
    fn an_undamped_lorentz_medium_is_sellmeiers() {
        let terms = [
            (0.6961663, 0.0684043),
            (0.4079426, 0.1162414),
            (0.8974794, 9.896161),
        ];
        let lorentz = Material::new(
            "silica as oscillators",
            Model::Lorentz {
                eps_inf: 1.0,
                poles: terms
                    .iter()
                    .map(|&(b, c)| LorentzPole {
                        strength: b,
                        resonance: Frequency::natural(1.0 / c).unwrap(),
                        damping: 0.0,
                    })
                    .collect(),
            },
            wavelength(0.21).unwrap(),
            wavelength(3.71).unwrap(),
            provenance("test"),
        )
        .unwrap();
        for l in [0.25, 0.6328, 1.31, 1.55, 3.5] {
            let w = wavelength(l).unwrap();
            close(
                lorentz.refractive_index(w).unwrap().re,
                silica().refractive_index(w).unwrap().re,
                1e-12,
            );
        }
    }

    /// The Cauchy fit reproduces a Cauchy formula exactly, holds Malitson's silica to five
    /// decimals inside its band, and leaves it beyond, where his infrared term bends the index
    /// down.
    #[test]
    fn the_cauchy_fit_holds_inside_and_fails_beyond() {
        let exact = [1.45, 0.0036, 0.0001];
        let points: Vec<[f64; 2]> = span(0.4, 1.0, 30)
            .into_iter()
            .map(|l| {
                let x = 1.0 / (l * l);
                [l, exact[0] + exact[1] * x + exact[2] * x * x]
            })
            .collect();
        for (a, b) in cauchy_fit(&points).iter().zip(exact) {
            close(*a, b, 1e-10);
        }
        let data = cauchy(0.7).unwrap();
        assert!(fig(&data, "the fit's largest error") < 1e-4);
        let at_3 = fig(&data, "error at 3.0 µm");
        assert!(at_3 > 1e-2, "{at_3}");
        let holds = fig(&data, "within $10^{-4}$");
        assert!(holds > 0.7 && holds < 0.8, "{holds}");
        // fitted further it holds no better: three Cauchy terms can't bend down as the
        // infrared term does
        assert!(fig(&cauchy(1.0).unwrap(), "the fit's largest error") > 1e-4);
        // the fitted curve's points are Model::Cauchy's, the other's Malitson's
        let m = series(&data, "Malitson's Sellmeier formula");
        for p in m.iter().step_by(37) {
            close(
                p[1],
                silica()
                    .refractive_index(wavelength(p[0]).unwrap())
                    .unwrap()
                    .re,
                0.0,
            );
        }
    }

    /// The index chart draws the catalogue's own n and n_g; n_g is n − λ dn/dλ, which the
    /// library's mode::dispersion::group_index finds from the same index sampled 1 nm apart.
    #[test]
    fn the_index_chart_is_the_catalogues() {
        for which in 1..=4 {
            let s = shown(which).unwrap();
            let data = index(which, 1.55).unwrap();
            let n = series(&data, "refractive index n");
            let ng = series(&data, "group index n_g");
            for (p, q) in n.iter().zip(ng).skip(3).step_by(41) {
                let l = p[0];
                close(
                    p[1],
                    s.material
                        .refractive_index(wavelength(l).unwrap())
                        .unwrap()
                        .re,
                    0.0,
                );
                let (lo, hi) = s.material.range();
                if l - 0.001 < lo.to_um() || l + 0.001 > hi.to_um() {
                    continue;
                }
                let ws: Vec<Wavelength> = [l - 0.001, l, l + 0.001]
                    .iter()
                    .map(|&x| wavelength(x).unwrap())
                    .collect();
                let ns: Vec<f64> = ws
                    .iter()
                    .map(|&w| s.material.refractive_index(w).unwrap().re)
                    .collect();
                close(q[1], dispersion::group_index(&ws, &ns).unwrap()[1], 2e-5);
            }
        }
    }

    /// The numbers the lesson quotes: silica's and silicon's n and n_g at 1.55 µm, and
    /// Leviton and Frey's absolute index above Malitson's by about the index of air.
    #[test]
    fn the_lessons_indices() {
        let silica = index(1, 1.55).unwrap();
        close(fig(&silica, "$n$ at 1.55"), 1.444024, 1e-6);
        close(fig(&silica, "$n_g$ at 1.55"), 1.4626, 1e-4);
        let si = index(2, 1.55).unwrap();
        close(fig(&si, "$n$ at 1.55"), 3.4757, 1e-12);
        close(fig(&si, "$n_g$ at 1.55"), 3.599, 1e-3);
        // Leviton and Frey: in vacuum, Malitson's in air (n_air ≈ 1.00027); within 1e-5 of
        // his formula so corrected, below 2 µm, as they report
        let lf = index(4, 1.55).unwrap();
        let above = fig(&lf, "minus Malitson's");
        // the index of air, and 1.85 K more at Malitson's 1e-5 per K (his Fig. 3)
        let air = 0.00027 * 1.444024 + 1.85e-5;
        close(above, air, 1e-5);
        // outside a model's range there is no index
        let out = index(2, 1.0).unwrap();
        assert!(out.figures[0].value.is_none() && out.marker.is_none());
    }

    /// D is −(λ/c) d²n/dλ²: on a parabola in λ the three-point difference is exact, and
    /// silica's D crosses zero near 1.27 µm.
    #[test]
    fn gvd_is_the_curvature_and_silicas_zero_is_near_1_27() {
        let parabola = Material::new(
            "parabola",
            Model::Formula {
                number: 3,
                // n² = C1 + C2 λ^C3 + ...: refractiveindex.info's formula 3, a test medium
                coefficients: vec![2.0, -0.01, 2.0],
            },
            wavelength(1.0).unwrap(),
            wavelength(3.0).unwrap(),
            provenance("test"),
        )
        .unwrap();
        let l = 1.55;
        let n = |x: f64| (2.0 - 0.01 * x * x).sqrt();
        let h = 1e-3;
        let d2 = (n(l + h) - 2.0 * n(l) + n(l - h)) / (h * h);
        let expect = -l * d2 / 299_792_458.0 * 1e12;
        close(at_d_at(&parabola, l).unwrap(), expect, 1e-2 * expect.abs());
        let data = gvd(1, 1.55).unwrap();
        let si = gvd(2, 1.55).unwrap();
        assert_eq!(si.series[0].label, SI_FREY);
        assert!(!si.series[0].dashed);
        assert_eq!(si.series[1].label, SI_LI);
        assert!(si.series[1].dashed);
        let z = fig(&data, "silica's zero dispersion");
        assert!((1.26..1.29).contains(&z), "{z}");
        let d = fig(&data, "silica's $D$");
        assert!(d > 15.0 && d < 30.0, "{d}");
        assert!(fig(&data, "silicon's $D$ at 1.55") < 0.0);
    }

    /// Silicon's default curve is the library's D (`mode::dispersion::dispersion`) on Frey et
    /// al.'s formula at 295 K, and smooth: its second difference along the curve is a small
    /// part of D, where Li's splined table jumps by hundreds of ps/(nm km) at its knots. The
    /// lesson quotes about −860 ps/(nm km) at 1.55 µm.
    #[test]
    fn silicons_d_is_freys_and_smooth() {
        let frey = silicon_frey().unwrap();
        let data = gvd(2, 1.55).unwrap();
        let curve = series(&data, SI_FREY);
        for p in curve.iter().step_by(29) {
            let ws: Vec<Wavelength> = [p[0] - 0.002, p[0], p[0] + 0.002]
                .iter()
                .map(|&x| wavelength(x).unwrap())
                .collect();
            let ns: Vec<f64> = ws
                .iter()
                .map(|&w| frey.refractive_index(w).unwrap().re)
                .collect();
            // the chart's neighbours are its own samples, 2 nm apart to rounding
            close(
                p[1],
                dispersion::dispersion(&ws, &ns).unwrap()[1],
                1e-7 * p[1].abs(),
            );
        }
        let d = fig(&data, "silicon's $D$ at 1.55");
        close(d, at_d_at(&frey, 1.55).unwrap(), 0.0);
        close(d, -862.1, 0.05);
        close(
            fig(&data, "from Li's table at 1.55"),
            at_d_at(&material::silicon(), 1.55).unwrap(),
            0.0,
        );
        let jump = |c: &[[f64; 2]]| {
            c.windows(3)
                .map(|w| (w[0][1] - 2.0 * w[1][1] + w[2][1]).abs())
                .fold(0.0, f64::max)
        };
        let smooth = jump(curve);
        assert!(smooth < 1.0, "{smooth}");
        let li = jump(series(&data, SI_LI));
        assert!(li > 10.0 * smooth, "{li} against {smooth}");
    }
}
