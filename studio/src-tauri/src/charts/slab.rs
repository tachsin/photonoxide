//! The slab waveguide's charts: a film of index n₂ and thickness d on a substrate n₃ under a
//! cover n₁, its TE and TM modes found exactly by `mode::slab` (the roots of Yariv and Yeh's
//! characteristic equations, bisected to the last bit), and the share of each mode's power in
//! the film by `mode::multilayer` (Chilwell and Hodgkinson's Eq. 42). Nothing here has a grid.
//!
//! The substrate is oxide, 1.444, and the cover between air and oxide, so the substrate is the
//! higher cladding and sets the cutoffs. The film is Chrostowski and Hochberg's silicon, 3.473
//! (Silicon Photonics Design, 2015, doi:10.1017/CBO9781316084168, Section 3.2.2), unless a chart
//! lets it change; the indices are held fixed over wavelength.
//!
//! The normalized quantities are Kogelnik and Ramaswamy's (Appl. Opt. 13, 1857 (1974),
//! doi:10.1364/AO.13.001857): V = k d √(n₂² − n₃²), b = (n_eff² − n₃²)/(n₂² − n₃²) and the
//! asymmetry a = (n₃² − n₁²)/(n₂² − n₃²). Mode m is cut off at n_eff = n₃, where the substrate's
//! decay rate p is zero and the transverse resonance h d = mπ + atan(q̄/h) + atan(p̄/h) leaves
//! V = mπ + atan(r √a), r = 1 for TE and (n₂/n₁)² for TM.

use std::f64::consts::PI;

use num_complex::Complex64 as c64;
use photonoxide::mode::Polarization;
use photonoxide::mode::multilayer::Multilayer;
use photonoxide::mode::slab::{Slab, SlabMode};
use photonoxide::units::{Length, Wavelength};

use super::{ChartData, Curve, Figure, figure, sig};

/// Chrostowski and Hochberg's silicon and oxide (Section 3.2.2).
pub const SILICON: f64 = 3.473;
pub const OXIDE: f64 = 1.444;
/// The orders drawn of each polarization in the b–V, index and confinement charts.
const ORDERS: usize = 4;
/// The orders drawn of each polarization in the field chart.
const FIELD_ORDERS: usize = 3;
/// The palette's grey, for reference lines.
const GREY: usize = 7;

/// A film between a cover and an oxide substrate.
#[derive(Clone, Copy, Debug)]
pub struct Film {
    pub cover: f64,
    pub core: f64,
}

impl Film {
    /// The film `d` µm thick.
    pub fn slab(&self, d: f64) -> photonoxide::Result<Slab> {
        Slab::new(self.cover, self.core, OXIDE, Length::um(d))
    }

    /// √(n₂² − n₃²).
    fn root(&self) -> f64 {
        (self.core * self.core - OXIDE * OXIDE).sqrt()
    }

    /// Kogelnik and Ramaswamy's asymmetry a = (n₃² − n₁²)/(n₂² − n₃²).
    pub fn asymmetry(&self) -> f64 {
        (OXIDE * OXIDE - self.cover * self.cover) / (self.core * self.core - OXIDE * OXIDE)
    }

    /// The factor TM's cover term carries, (n₂/n₁)²; 1 for TE.
    fn factor(&self, pol: Polarization) -> f64 {
        match pol {
            Polarization::Te => 1.0,
            Polarization::Tm => (self.core / self.cover).powi(2),
        }
    }

    /// The V below which mode `m` of `pol` isn't guided: mπ + atan(r √a).
    pub fn cutoff_v(&self, pol: Polarization, m: usize) -> f64 {
        m as f64 * PI + (self.factor(pol) * self.asymmetry().sqrt()).atan()
    }

    /// The thickness, µm, below which mode `m` isn't guided at `l` µm.
    pub fn cutoff_thickness(&self, pol: Polarization, m: usize, l: f64) -> f64 {
        self.cutoff_v(pol, m) / (2.0 * PI / l * self.root())
    }

    /// V of a film `d` µm thick at `l` µm.
    pub fn v(&self, d: f64, l: f64) -> f64 {
        2.0 * PI / l * d * self.root()
    }

    /// b of an effective index.
    pub fn b(&self, n: f64) -> f64 {
        (n * n - OXIDE * OXIDE) / (self.core * self.core - OXIDE * OXIDE)
    }

    /// The film's index at a height `x` µm above the substrate's face, `d` µm thick: the
    /// substrate's below 0 and the cover's above d (a face counts as the film).
    fn index_at(&self, x: f64, d: f64) -> f64 {
        if x < 0.0 {
            OXIDE
        } else if x > d {
            self.cover
        } else {
            self.core
        }
    }
}

fn wavelength(um: f64) -> photonoxide::Result<Wavelength> {
    Wavelength::um(um)
}

fn pol_name(pol: Polarization) -> &'static str {
    match pol {
        Polarization::Te => "TE",
        Polarization::Tm => "TM",
    }
}

/// A mode's name, TE₀ or TM₁.
fn name(pol: Polarization, m: usize) -> String {
    const SUB: [char; 10] = ['₀', '₁', '₂', '₃', '₄', '₅', '₆', '₇', '₈', '₉'];
    let digits: String = m
        .to_string()
        .chars()
        .map(|c| SUB[c.to_digit(10).unwrap_or(0) as usize])
        .collect();
    format!("{}{digits}", pol_name(pol))
}

/// A mode's curve: TE solid and TM dashed, in the colour of its order.
fn mode_curve(pol: Polarization, m: usize, points: Vec<[f64; 2]>) -> Curve {
    Curve {
        label: name(pol, m),
        points,
        dashed: pol == Polarization::Tm,
        colour: Some(m),
    }
}

/// A grey reference line from (x0, y0) to (x1, y1), with points along it.
fn line(label: &str, from: [f64; 2], to: [f64; 2]) -> Curve {
    let points = (0..=20)
        .map(|k| {
            let t = f64::from(k) / 20.0;
            [
                from[0] + t * (to[0] - from[0]),
                from[1] + t * (to[1] - from[1]),
            ]
        })
        .collect();
    Curve {
        label: label.to_owned(),
        points,
        dashed: true,
        colour: Some(GREY),
    }
}

const POLS: [Polarization; 2] = [Polarization::Te, Polarization::Tm];

/// Each mode's points, TE and TM, orders below `ORDERS`: `at(x)` gives the x to sample and the
/// value of a mode; each curve starts at its cutoff, `(cutoff(pol, m), y_cut)`, where that is
/// inside `[0, x_max]`.
fn by_mode(
    xs: &[f64],
    modes: impl Fn(f64, Polarization) -> photonoxide::Result<Vec<f64>>,
    cutoff: impl Fn(Polarization, usize) -> f64,
    y_cut: f64,
    x_max: f64,
) -> photonoxide::Result<Vec<Curve>> {
    let mut curves = Vec::new();
    for pol in POLS {
        let mut points: Vec<Vec<[f64; 2]>> = (0..ORDERS)
            .map(|m| {
                let c = cutoff(pol, m);
                if c < x_max {
                    vec![[c, y_cut]]
                } else {
                    Vec::new()
                }
            })
            .collect();
        for &x in xs {
            for (m, y) in modes(x, pol)?.into_iter().take(ORDERS).enumerate() {
                points[m].push([x, y]);
            }
        }
        for (m, p) in points.into_iter().enumerate() {
            if p.len() > 2 {
                curves.push(mode_curve(pol, m, p));
            }
        }
    }
    // TE₀, TM₀, TE₁, TM₁, …: each order's pair together in the legend
    curves.sort_by_key(|c| (c.colour, c.dashed));
    Ok(curves)
}

/// The b–V diagram of a film between `cover` and oxide.
pub fn bv(core: f64, cover: f64) -> photonoxide::Result<ChartData> {
    let film = Film { cover, core };
    const V_MAX: f64 = 12.0;
    // V is a thickness at any wavelength: 1 µm here
    let l = wavelength(1.0)?;
    let vs: Vec<f64> = (1..=300).map(|k| V_MAX * f64::from(k) / 300.0).collect();
    let series = by_mode(
        &vs,
        |v, pol| {
            let d = v / film.v(1.0, 1.0);
            Ok(film
                .slab(d)?
                .modes(pol, l)
                .iter()
                .map(|m| film.b(m.effective_index()))
                .collect())
        },
        |pol, m| film.cutoff_v(pol, m),
        0.0,
        V_MAX,
    )?;
    let a = film.asymmetry();
    let r = film.factor(Polarization::Tm);
    let single_te = film.cutoff_v(Polarization::Te, 1);
    let figures = vec![
        figure(
            "asymmetry $a$",
            a,
            4,
            "",
            if a == 0.0 { "a symmetric film" } else { "" },
        ),
        figure(
            "TE₀ cut off below $V$",
            film.cutoff_v(Polarization::Te, 0),
            4,
            "",
            "$\\arctan\\sqrt a$",
        ),
        figure(
            "TM₀ cut off below $V$",
            film.cutoff_v(Polarization::Tm, 0),
            4,
            "",
            "$\\arctan\\left((n_2/n_1)^2\\sqrt a\\right)$",
        ),
        figure(
            "one TE mode below $V$",
            single_te,
            4,
            "",
            "TE₁'s cutoff, $\\pi + \\arctan\\sqrt a$",
        ),
        figure(
            "one TM mode below $V$",
            film.cutoff_v(Polarization::Tm, 1),
            4,
            "",
            "TM₁'s cutoff",
        ),
        figure(
            "TM's factor $(n_2/n_1)^2$",
            r,
            4,
            "",
            "1 would make TM's curves TE's",
        ),
    ];
    Ok(ChartData {
        x_label: "normalized frequency V".to_owned(),
        x_length: false,
        y_label: "normalized index b".to_owned(),
        y_log: false,
        y_range: Some([-0.02, 1.02]),
        series,
        marker: Some(single_te),
        figures,
        note: format!(
            "Exact, no grid: the roots of the slab's characteristic equations for a film of {core} on oxide (1.444) under a cover of {cover}. TE's curves depend on the asymmetry a alone; TM's on the indices too. The line marks TE₁'s cutoff, the end of single-mode operation."
        ),
    })
}

/// A field across the film at height `x` µm above the substrate: E_y for TE, E_x for TM
/// (H_y/n², which jumps at the faces where H_y doesn't), unscaled.
fn transverse(film: &Film, mode: &SlabMode, d: f64, x: f64) -> f64 {
    // the slab's own x runs from its cover's face (0) down into the substrate (d and beyond)
    let (v, _) = mode.field(Length::um(d - x));
    match mode.polarization() {
        Polarization::Te => v,
        Polarization::Tm => v / film.index_at(x, d).powi(2),
    }
}

/// Scales points to a largest magnitude of 1, positive.
fn scaled(mut points: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    let peak = points
        .iter()
        .map(|p| p[1])
        .fold(0.0f64, |a, y| if y.abs() > a.abs() { y } else { a });
    if peak != 0.0 {
        for p in &mut points {
            p[1] /= peak;
        }
    }
    points
}

/// The heights to draw a field at, from `-span` to `d + span` µm: each face twice, the medium
/// below it and the film's, so that a jump shows as one.
fn heights(d: f64, span: f64) -> Vec<(f64, bool)> {
    let mut out = Vec::new();
    let n = 120;
    // (x, whether it is taken on the outer side of a face)
    for k in 0..=n {
        let x = -span + span * f64::from(k) / f64::from(n);
        out.push((x, k == n));
    }
    for k in 0..=n {
        out.push((d * f64::from(k) / f64::from(n), false));
    }
    for k in 0..=n {
        out.push((d + span * f64::from(k) / f64::from(n), k == 0));
    }
    out
}

/// The guided modes' fields across a silicon film `d_nm` thick on oxide under `cover`, at `l` µm.
pub fn fields(d_nm: f64, l: f64, cover: f64) -> photonoxide::Result<ChartData> {
    let film = Film {
        cover,
        core: SILICON,
    };
    let d = d_nm * 1e-3;
    let slab = film.slab(d)?;
    let w = wavelength(l)?;
    let te = slab.modes(Polarization::Te, w);
    let tm = slab.modes(Polarization::Tm, w);
    // the tails out to three decay lengths of the slowest, within 0.3 to 3 µm
    let drawn = || te.iter().chain(&tm).filter(|m| m.order() < FIELD_ORDERS);
    let longest = drawn()
        .map(|m| {
            let (_, q, p) = m.wavenumbers();
            1.0 / q.min(p)
        })
        .fold(0.0f64, f64::max);
    let span = (3.0 * longest).clamp(0.3, 3.0);
    let xs = heights(d, span);
    let mut series = Vec::new();
    for mode in drawn() {
        let points = xs
            .iter()
            .map(|&(x, outer)| {
                // at a face, the outer side takes the outer medium's index
                let at = if outer {
                    if x <= 0.0 { x - 1e-12 } else { x + 1e-12 }
                } else {
                    x
                };
                [x, transverse(&film, mode, d, at)]
            })
            .collect();
        series.push(mode_curve(
            mode.polarization(),
            mode.order(),
            scaled(points),
        ));
    }
    series.sort_by_key(|c| (c.colour, c.dashed));
    // the film's faces, as a box
    let top = 1.1;
    let mut outline = line("film, 0 < x < d", [0.0, -top], [0.0, top]).points;
    outline.extend(line("", [0.0, top], [d, top]).points);
    outline.extend(line("", [d, top], [d, -top]).points);
    series.push(Curve {
        label: "film, 0 < x < d".to_owned(),
        points: outline,
        dashed: true,
        colour: Some(GREY),
    });
    let mut figures = Vec::new();
    for mode in te
        .iter()
        .take(2)
        .zip(tm.iter().take(2))
        .flat_map(|(a, b)| [a, b])
    {
        figures.push(figure(
            &format!(
                "{} $n_\\text{{eff}}$",
                name(mode.polarization(), mode.order())
            ),
            mode.effective_index(),
            6,
            "",
            "",
        ));
    }
    figures.push(Figure {
        label: "guided modes".to_owned(),
        value: Some((te.len() + tm.len()) as f64),
        text: format!("{} TE, {} TM", te.len(), tm.len()),
        unit: String::new(),
        note: if te.len() > FIELD_ORDERS || tm.len() > FIELD_ORDERS {
            format!("the first {FIELD_ORDERS} of each drawn")
        } else {
            String::new()
        },
    });
    figures.push(figure("$V$", film.v(d, l), 4, "", ""));
    if let Some(m) = te.first() {
        let (_, q, p) = m.wavenumbers();
        figures.push(figure(
            "TE₀'s tail in the cover, $1/q$",
            1e3 / q,
            4,
            "nm",
            "the field falls by e",
        ));
        figures.push(figure(
            "TE₀'s tail in the substrate, $1/p$",
            1e3 / p,
            4,
            "nm",
            "",
        ));
    }
    Ok(ChartData {
        x_label: "height above the substrate x".to_owned(),
        x_length: true,
        y_label: "field, scaled to its peak".to_owned(),
        y_log: false,
        y_range: Some([-1.15, 1.15]),
        series,
        marker: None,
        figures,
        note: format!(
            "Exact, no grid: silicon (3.473) on oxide (1.444) under a cover of {cover}, Chrostowski and Hochberg's indices held fixed over wavelength. TE modes' E_y solid, TM modes' E_x dashed: E_x jumps at each face by the ratio of the two permittivities. Each field is scaled to its largest value."
        ),
    })
}

/// The heights the index and confinement charts sample, from 0 to `x_max` µm.
fn thicknesses(x_max: f64, n: u32) -> Vec<f64> {
    (1..=n)
        .map(|k| x_max * f64::from(k) / f64::from(n))
        .collect()
}

/// The effective indices of a film between `cover` and oxide against its thickness, at `l` µm.
pub fn neff(core: f64, cover: f64, l: f64) -> photonoxide::Result<ChartData> {
    let film = Film { cover, core };
    let w = wavelength(l)?;
    // out to V = 10.5: four orders of each in a symmetric film
    let x_max = 10.5 / film.v(1.0, l);
    let mut series = by_mode(
        &thicknesses(x_max, 300),
        |d, pol| {
            Ok(film
                .slab(d)?
                .modes(pol, w)
                .iter()
                .map(SlabMode::effective_index)
                .collect())
        },
        |pol, m| film.cutoff_thickness(pol, m, l),
        OXIDE,
        x_max,
    )?;
    series.push(line("film n₂", [0.0, core], [x_max, core]));
    series.push(line("substrate n₃", [0.0, OXIDE], [x_max, OXIDE]));
    let nm = |pol, m| 1e3 * film.cutoff_thickness(pol, m, l);
    let single = film.cutoff_thickness(Polarization::Te, 1, l);
    let figures = vec![
        figure(
            "TE₀ guided above",
            nm(Polarization::Te, 0),
            4,
            "nm",
            "0 in a symmetric film",
        ),
        figure("TM₀ guided above", nm(Polarization::Tm, 0), 4, "nm", ""),
        figure(
            "one TE mode below",
            nm(Polarization::Te, 1),
            4,
            "nm",
            "TE₁'s cutoff, marked",
        ),
        figure("one TM mode below", nm(Polarization::Tm, 1), 4, "nm", ""),
        figure("TE₂ guided above", nm(Polarization::Te, 2), 4, "nm", ""),
    ];
    let pad = 0.03 * (core - OXIDE);
    Ok(ChartData {
        x_label: "film thickness d".to_owned(),
        x_length: true,
        y_label: "effective index n_eff".to_owned(),
        y_log: false,
        y_range: Some([OXIDE - pad, core + pad]),
        series,
        marker: Some(single),
        figures,
        note: format!(
            "Exact, no grid: a film of {core} on oxide (1.444) under a cover of {cover} at {} µm. Each mode starts at its cutoff, where its index reaches the substrate's, from mπ + atan(r√a) with r = 1 for TE and (n₂/n₁)² for TM.",
            sig(l, 4)
        ),
    })
}

/// A mode's share of its power in the film, and the field's decay lengths in the cover and the
/// substrate (µm): the slab's exact mode, its power by the transfer matrices.
pub fn confinement_of(
    film: &Film,
    d: f64,
    l: f64,
    mode: &SlabMode,
) -> photonoxide::Result<(f64, f64, f64)> {
    let stack = Multilayer::new(
        c64::new(film.cover, 0.0),
        &[(c64::new(film.core, 0.0), Length::um(d))],
        c64::new(OXIDE, 0.0),
    )?;
    let m = stack.mode_near(
        mode.polarization(),
        wavelength(l)?,
        c64::new(mode.effective_index(), 0.0),
    )?;
    let (_, q, p) = mode.wavenumbers();
    Ok((m.power_fractions()[1], 1.0 / q, 1.0 / p))
}

/// The share of each mode's power in a film between `cover` and oxide against its thickness, at
/// `l` µm, and its tails at `at_nm`.
pub fn confinement(core: f64, cover: f64, l: f64, at_nm: f64) -> photonoxide::Result<ChartData> {
    let film = Film { cover, core };
    let at = at_nm * 1e-3;
    let x_max = (7.0 / film.v(1.0, l)).max(1.5 * at);
    let gamma = |d: f64, pol| -> photonoxide::Result<Vec<f64>> {
        film.slab(d)?
            .modes(pol, wavelength(l)?)
            .iter()
            .take(2)
            .map(|m| Ok(confinement_of(&film, d, l, m)?.0))
            .collect()
    };
    let series: Vec<Curve> = by_mode(
        &thicknesses(x_max, 200),
        gamma,
        |pol, m| film.cutoff_thickness(pol, m, l),
        0.0,
        x_max,
    )?
    .into_iter()
    .filter(|c| c.colour.is_some_and(|m| m < 2))
    .collect();
    let slab = film.slab(at)?;
    let mut figures = Vec::new();
    for pol in POLS {
        let n = name(pol, 0);
        match slab.modes(pol, wavelength(l)?).first() {
            Some(m) => {
                let (g, cover_tail, substrate_tail) = confinement_of(&film, at, l, m)?;
                figures.push(figure(&format!("{n} in the film, Γ"), g, 4, "", ""));
                figures.push(figure(
                    &format!("{n}'s tail in the cover, $1/q$"),
                    1e3 * cover_tail,
                    4,
                    "nm",
                    "",
                ));
                figures.push(figure(
                    &format!("{n}'s tail in the substrate, $1/p$"),
                    1e3 * substrate_tail,
                    4,
                    "nm",
                    "",
                ));
            }
            None => figures.push(Figure {
                label: format!("{n} in the film, Γ"),
                value: None,
                text: "cut off".to_owned(),
                unit: String::new(),
                note: String::new(),
            }),
        }
    }
    Ok(ChartData {
        x_label: "film thickness d".to_owned(),
        x_length: true,
        y_label: "share of the power in the film, Γ".to_owned(),
        y_log: false,
        y_range: Some([-0.02, 1.02]),
        series,
        marker: Some(at),
        figures,
        note: format!(
            "Exact, no grid: a film of {core} on oxide (1.444) under a cover of {cover} at {} µm. Γ is the share of the mode's power, the Poynting vector along the film, between its faces (Chilwell and Hodgkinson's Eq. 42); the tails are where the field has fallen by e at the marked thickness.",
            sig(l, 4)
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "{a} against {b}");
    }

    fn curve<'a>(data: &'a ChartData, label: &str) -> &'a Curve {
        data.series
            .iter()
            .find(|c| c.label == label)
            .unwrap_or_else(|| panic!("no curve {label}"))
    }

    /// TE's b on the chart solves Kogelnik and Ramaswamy's normalized equation,
    /// V√(1 − b) = mπ + atan√(b/(1 − b)) + atan√((b + a)/(1 − b)), and TM's the same with the
    /// cover's and substrate's terms multiplied by (n₂/n₁)² and (n₂/n₃)².
    #[test]
    fn bv_solves_the_normalized_equation() {
        for (core, cover) in [(3.473, 1.444), (3.473, 1.0), (1.6, 1.2), (2.0, 1.0)] {
            let film = Film { cover, core };
            let a = film.asymmetry();
            let data = bv(core, cover).unwrap();
            for c in &data.series {
                let Some(m) = c.colour.filter(|&m| m < ORDERS) else {
                    continue;
                };
                let (rc, rs) = if c.dashed {
                    ((core / cover).powi(2), (core / OXIDE).powi(2))
                } else {
                    (1.0, 1.0)
                };
                for &[v, b] in c.points.iter().skip(1) {
                    let lhs = v * (1.0 - b).sqrt();
                    let rhs = m as f64 * PI
                        + (rs * (b / (1.0 - b)).sqrt()).atan()
                        + (rc * ((b + a) / (1.0 - b)).sqrt()).atan();
                    close(lhs, rhs, 1e-9);
                }
            }
        }
    }

    /// A symmetric film's TE curves are the same whatever its indices; its TM curves aren't,
    /// and fall further below TE's the higher the contrast.
    #[test]
    fn te_is_universal_and_tm_falls_with_contrast() {
        let weak = bv(1.5, OXIDE).unwrap();
        let strong = bv(3.473, OXIDE).unwrap();
        let at = |data: &ChartData, label: &str, v: f64| {
            let p = curve(data, label)
                .points
                .iter()
                .min_by(|a, b| (a[0] - v).abs().total_cmp(&(b[0] - v).abs()))
                .unwrap();
            assert!((p[0] - v).abs() < 1e-9);
            p[1]
        };
        for v in [1.0, 2.0, 4.0, 8.0] {
            close(at(&weak, "TE₀", v), at(&strong, "TE₀", v), 1e-12);
        }
        let gap = |data: &ChartData| at(data, "TE₀", 2.0) - at(data, "TM₀", 2.0);
        assert!(gap(&weak) > 0.0 && gap(&weak) < 0.03, "{}", gap(&weak));
        assert!(gap(&strong) > 0.2, "{}", gap(&strong));
    }

    /// Each curve starts at its cutoff: the slab guides mode m just above it and not just below.
    #[test]
    fn the_cutoffs_are_the_slabs() {
        for (core, cover) in [(3.473, 1.0), (2.0, 1.2), (1.6, 1.444)] {
            let film = Film { cover, core };
            let l = wavelength(1.55).unwrap();
            for pol in POLS {
                for m in 0..3 {
                    let d = film.cutoff_thickness(pol, m, 1.55);
                    if d == 0.0 {
                        continue;
                    }
                    let count = |d: f64| film.slab(d).unwrap().modes(pol, l).len();
                    assert_eq!(count(d * (1.0 + 1e-6)), m + 1, "{pol:?}{m} above");
                    assert_eq!(count(d * (1.0 - 1e-6)), m, "{pol:?}{m} below");
                }
            }
        }
    }

    /// The index chart's points are the slab's modes; its figures, the cutoffs of SOI under air.
    #[test]
    fn neff_is_the_slabs_and_soi_under_air_has_its_cutoffs() {
        let data = neff(SILICON, 1.0, 1.55).unwrap();
        let film = Film {
            cover: 1.0,
            core: SILICON,
        };
        let w = wavelength(1.55).unwrap();
        for (label, pol, m) in [("TE₀", Polarization::Te, 0), ("TM₁", Polarization::Tm, 1)] {
            for &[d, n] in curve(&data, label).points.iter().skip(1).step_by(7) {
                let exact = film.slab(d).unwrap().modes(pol, w)[m].effective_index();
                close(n, exact, 0.0);
            }
        }
        let fig = |label: &str| {
            data.figures
                .iter()
                .find(|f| f.label == label)
                .unwrap()
                .value
                .unwrap()
        };
        // a = (1.444² − 1)/(3.473² − 1.444²); atan √a / (k √(n₂² − n₃²))
        close(fig("TE₀ guided above"), 24.88, 0.01);
        close(fig("TM₀ guided above"), 103.4, 0.1);
        // symmetric: one TE mode below π / (k √(n₂² − n₃²)) = 245.4 nm
        let sym = neff(SILICON, OXIDE, 1.55).unwrap();
        let single = sym.marker.unwrap();
        close(single * 1e3, 245.4, 0.05);
    }

    /// The fields are the slab's: E_y for TE; for TM, H_y over n², which jumps at a face by the
    /// ratio of the permittivities.
    #[test]
    fn the_fields_are_the_slabs_and_tms_jumps() {
        let data = fields(220.0, 1.55, 1.0).unwrap();
        let film = Film {
            cover: 1.0,
            core: SILICON,
        };
        let slab = film.slab(0.22).unwrap();
        let te = slab.modes(Polarization::Te, wavelength(1.55).unwrap())[0];
        let points = &curve(&data, "TE₀").points;
        let peak = points
            .iter()
            .map(|p| transverse(&film, &te, 0.22, p[0]))
            .fold(0.0f64, |a, v| if v.abs() > a.abs() { v } else { a });
        for &[x, y] in points.iter().step_by(11) {
            // a face's outer point is taken 1e-12 µm outside it
            close(y, transverse(&film, &te, 0.22, x) / peak, 1e-9);
        }
        let tm = &curve(&data, "TM₀").points;
        // the points either side of each face
        let jump = |x: f64| {
            let side: Vec<f64> = tm
                .iter()
                .filter(|p| (p[0] - x).abs() < 1e-15)
                .map(|p| p[1])
                .collect();
            assert_eq!(side.len(), 2, "two points at {x}");
            side[0] / side[1]
        };
        // E_x outside over E_x in the film: (n₂/n_out)²
        close(jump(0.0), (SILICON / OXIDE).powi(2), 1e-9);
        close(1.0 / jump(0.22), (SILICON / 1.0).powi(2), 1e-9);
    }

    /// Γ from the transfer matrices' power is the slab's exact field integrated: |E_y|² for TE
    /// and |H_y|²/n² for TM, across the film over the whole, the tails in closed form.
    #[test]
    fn confinement_is_the_exact_fields_power() {
        for (cover, d) in [(OXIDE, 0.22), (1.0, 0.22), (1.0, 0.6)] {
            let film = Film {
                cover,
                core: SILICON,
            };
            let slab = film.slab(d).unwrap();
            for pol in POLS {
                for mode in slab.modes(pol, wavelength(1.55).unwrap()) {
                    let w = |n: f64| match pol {
                        Polarization::Te => 1.0,
                        Polarization::Tm => 1.0 / (n * n),
                    };
                    let f = |x: f64| mode.field(Length::um(x)).0;
                    let steps = 4000;
                    let h = d / f64::from(steps);
                    let core: f64 = (0..=steps)
                        .map(|j| {
                            let c = if j == 0 || j == steps {
                                1.0
                            } else if j % 2 == 1 {
                                4.0
                            } else {
                                2.0
                            };
                            c * f(h * f64::from(j)).powi(2)
                        })
                        .sum::<f64>()
                        * h
                        / 3.0
                        * w(SILICON);
                    let (_, q, p) = mode.wavenumbers();
                    // the slab's x = 0 is the cover's face, x = d the substrate's
                    let tails =
                        f(0.0).powi(2) * w(cover) / (2.0 * q) + f(d).powi(2) * w(OXIDE) / (2.0 * p);
                    let (g, _, _) = confinement_of(&film, d, 1.55, &mode).unwrap();
                    close(g, core / (core + tails), 1e-6);
                }
            }
        }
    }

    /// The SOI slab's numbers the lesson quotes: TE₀ and TM₀ of 220 nm of silicon in oxide.
    #[test]
    fn soi_in_oxide() {
        let data = confinement(SILICON, OXIDE, 1.55, 220.0).unwrap();
        let fig = |label: &str| {
            data.figures
                .iter()
                .find(|f| f.label.starts_with(label))
                .unwrap()
                .value
                .unwrap()
        };
        close(fig("TE₀ in the film"), 0.810, 5e-4);
        close(fig("TM₀ in the film"), 0.583, 5e-4);
        close(fig("TE₀'s tail in the cover"), 100.6, 0.05);
        close(fig("TM₀'s tail in the substrate"), 169.4, 0.05);
    }
}
