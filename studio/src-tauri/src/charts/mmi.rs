//! The MMI lesson's charts: a multimode interference coupler by Soldano and Pennings's
//! guided-mode propagation analysis (J. Lightwave Technol. 13, 615 (1995),
//! doi:10.1109/50.372474, Section III), as photonoxide's MMI devices compute it in 2D
//! (`circuit::components::Mmi`, the model `device::StripMmi` builds): the plane seen from above
//! by the effective index method, a film of Li's silicon in Malitson's silica, its TE index the
//! ridge's and silica's the cladding's, and the exact slab modes of the multimode section and of
//! the access guides across it. No grid: each mode is exact, each overlap integrated by
//! Simpson's rule. 2D throughout, an estimate of the device, never its performance.
//!
//! The analysis is computed once for a width and wavelength (`Mmi::analysis`), so a sweep of the
//! section's length costs one solve, and the field inside (`MmiModes::field`) is the guided
//! modes' sum.

use photonoxide::portable;
use std::f64::consts::PI;

use photonoxide::circuit::components::{Mmi, MmiKind, MmiModes, Planar};
use photonoxide::material::{silica, silicon};
use photonoxide::mode::Polarization;
use photonoxide::mode::slab::Slab;
use photonoxide::raster::Raster;
use photonoxide::units::{Length, Wavelength};

use super::{ChartData, Curve, FieldMap, figure, invalid, sig};

/// The design wavelength, µm.
pub const DESIGN: f64 = 1.55;

/// The plane of a film `thickness_nm` thick of silicon in silica, as `device::StripMmi` builds
/// it on 220 nm SOI: the film's TE index the ridge's (TM across the section).
pub fn film(thickness_nm: f64) -> Planar {
    Planar::Film {
        core: silicon(),
        cladding: silica(),
        thickness: Length::nm(thickness_nm),
        vertical: Polarization::Te,
    }
}

/// The film's TE index and silica's at `l` µm: Soldano and Pennings's n_r and n_c.
pub fn indices(thickness_nm: f64, l: f64) -> photonoxide::Result<(f64, f64)> {
    let lam = Wavelength::um(l)?;
    let n = |m: &photonoxide::material::Material| -> photonoxide::Result<f64> {
        Ok(m.refractive_index(lam)?.re)
    };
    let (nf, nc) = (n(&silicon())?, n(&silica())?);
    let nr = Slab::new(nc, nf, nc, Length::nm(thickness_nm))?
        .modes(Polarization::Te, lam)
        .first()
        .map(|m| m.effective_index())
        .ok_or_else(|| invalid("thickness", "the film guides no mode"))?;
    Ok((nr, nc))
}

/// Soldano and Pennings's effective width, their Eq. 4 with σ = 1 (TM across the section):
/// W_e = W + (λ/π)(n_c/n_r)² (n_r² − n_c²)^(−1/2).
pub fn effective_width(width: f64, l: f64, nr: f64, nc: f64) -> f64 {
    width + l / PI * portable::powi(nc / nr, 2) / (nr * nr - nc * nc).sqrt()
}

/// The kind a chart's `kind` (1 or 2) names.
fn kind_of(kind: f64) -> photonoxide::Result<MmiKind> {
    match kind.round() as i64 {
        1 => Ok(MmiKind::OneByTwo),
        2 => Ok(MmiKind::TwoByTwo),
        _ => Err(invalid("kind", "1 for the 1 × 2, 2 for the 2 × 2")),
    }
}

/// The MMI of `kind`, `width` µm wide with access guides `access` µm wide on a film
/// `thickness_nm` thick, its length the first two-fold image at 1.55 µm.
pub fn mmi(kind: MmiKind, width: f64, access: f64, thickness_nm: f64) -> photonoxide::Result<Mmi> {
    Mmi::new(
        kind,
        film(thickness_nm),
        width,
        access,
        Wavelength::um(DESIGN)?,
    )
}

/// Where `f` is largest in [lo, hi]: the best of 61 even samples, refined by golden section
/// between its neighbours.
fn best_length(f: impl Fn(f64) -> f64, lo: f64, hi: f64) -> f64 {
    let step = (hi - lo) / 60.0;
    let z = (0..=60)
        .map(|k| lo + step * f64::from(k))
        .map(|z| (z, f(z)))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map_or(lo, |(z, _)| z);
    super::peak(f, (z - step).max(lo), (z + step).min(hi))
}

/// The length near `design` where the weaker output takes the most: the length a designer
/// cuts the section to, balance first (a 1 × 2 is balanced at every length, by symmetry).
fn best(a: &MmiModes, kind: MmiKind, design: f64) -> f64 {
    let weaker = |z: f64| {
        let (pa, pb, _) = split(a, kind, z);
        pa.min(pb)
    };
    best_length(weaker, 0.7 * design, 1.3 * design)
}

/// The first two-fold image's length, the MMI's own default.
fn design_length(m: &Mmi) -> f64 {
    photonoxide::circuit::Component::defaults(m)[0]
}

/// The ports a chart reads from the first input: (the upper or cross output, the lower or bar
/// output), and their names.
fn outputs(kind: MmiKind) -> ((usize, usize), [&'static str; 2]) {
    match kind {
        MmiKind::OneByTwo => ((1, 2), ["upper output", "lower output"]),
        MmiKind::TwoByTwo => ((2, 3), ["cross output", "bar output"]),
    }
}

/// The two outputs' powers from the first input, and their phase difference, degrees.
fn split(a: &MmiModes, kind: MmiKind, length: f64) -> (f64, f64, f64) {
    let ((p, q), _) = outputs(kind);
    let s = a.s_matrix(length);
    let (sp, sq) = (s[(p, 0)], s[(q, 0)]);
    (
        sp.norm_sqr(),
        sq.norm_sqr(),
        tidy(portable::complex::arg(sp / sq).to_degrees()),
    )
}

/// A number that is zero by symmetry, its round-off (below 1e-9) shown as the zero it is.
fn tidy(x: f64) -> f64 {
    if x.abs() < 1e-9 { 0.0 } else { x }
}

fn excess_db(pa: f64, pb: f64) -> f64 {
    -10.0 * portable::log10(pa + pb)
}

fn imbalance_db(pa: f64, pb: f64) -> f64 {
    tidy(10.0 * portable::log10(pa / pb))
}

fn curve(label: &str, points: Vec<[f64; 2]>, dashed: bool, colour: usize) -> Curve {
    Curve {
        label: label.to_owned(),
        points,
        dashed,
        colour: Some(colour),
    }
}

/// The beat length π/(β₀ − β₁) of an analysis, µm.
fn beat(a: &MmiModes) -> f64 {
    let b = a.propagation_constants();
    PI / (b[0] - b[1])
}

/// The light inside a section `width` µm wide, fed by a 0.8 µm guide whose centre is `offset`
/// of the width off the axis, over `span` beat lengths: |Σ c_ν ψ_ν e^{iβ_ν z}|² from above.
pub fn field(width: f64, offset: f64, span: f64) -> photonoxide::Result<ChartData> {
    const ACCESS: f64 = 0.8;
    let l = Wavelength::um(DESIGN)?;
    let m = mmi(MmiKind::OneByTwo, width, ACCESS, 220.0)?;
    let y_in = offset * width;
    let a = m.analysis_at(l, width, &[(y_in, true)]).map_err(|_| {
        invalid(
            "offset",
            format!(
                "the 0.8 µm guide must sit on the section's face: |offset| at most {}",
                sig((width - ACCESS) / 2.0 / width, 3)
            ),
        )
    })?;
    let lpi = beat(&a);
    let z1 = span * lpi;
    let margin = 0.4;
    let (y0, y1) = (-width / 2.0 - margin, width / 2.0 + margin);
    // a long section is drawn in rows, each at most eight times as long as it is tall, so that
    // its pictures stay tall enough to read at their true aspect
    let rows = ((z1 / (y1 - y0)) / 8.0).ceil().max(1.0) as usize;
    let (nx, ny) = (600, 150);
    let mut pictures = Vec::with_capacity(rows);
    for r in 0..rows {
        let (za, zb) = (
            z1 * r as f64 / rows as f64,
            z1 * (r + 1) as f64 / rows as f64,
        );
        let mut values = Vec::with_capacity(nx * ny);
        for j in 0..ny {
            let y = y0 + (y1 - y0) * (j as f64 + 0.5) / ny as f64;
            for i in 0..nx {
                let z = za + (zb - za) * (i as f64 + 0.5) / nx as f64;
                values.push(a.field(0, y, z).norm_sqr());
            }
        }
        pictures.push((za, zb, values));
    }
    // one colour scale for every row
    let top = pictures
        .iter()
        .flat_map(|p| p.2.iter().copied())
        .fold(0.0, f64::max)
        .max(f64::MIN_POSITIVE);
    let hw = width / 2.0;
    let c = a.couplings(0);
    let share = |pick: &dyn Fn(usize) -> bool| -> f64 {
        c.iter()
            .enumerate()
            .filter(|(v, _)| pick(*v))
            .map(|(_, c)| c * c)
            .sum()
    };
    let guided = share(&|_| true);
    // a centred input's odd share is round-off: shown as the zero it is
    let odd = share(&|v| v % 2 == 1);
    let odd = if odd < 1e-15 { 0.0 } else { odd };
    let third = share(&|v| v % 3 == 2);
    let mechanism = if odd < 1e-6 {
        "symmetric interference: single images at 3L_π/4, N-fold at 3L_π/4N"
    } else if third < 0.01 * guided {
        "paired interference: single images at L_π, two-fold at L_π/2"
    } else {
        "general interference: single images at 3L_π, two-fold at 3L_π/2"
    };
    Ok(ChartData {
        maps: pictures
            .into_iter()
            .map(|(za, zb, values)| {
                let mut outline = vec![vec![[za, -hw], [zb, -hw]], vec![[za, hw], [zb, hw]]];
                if za == 0.0 {
                    // the input face, and the input guide's edges on it
                    outline.push(vec![[0.0, -hw], [0.0, hw]]);
                    outline.push(vec![[0.0, y_in - ACCESS / 2.0], [0.0, y_in + ACCESS / 2.0]]);
                }
                FieldMap {
                    label: if rows == 1 {
                        format!(
                            "$|\\psi(y, z)|^2$ from above, the input at $y = {}$ µm",
                            sig(y_in, 3)
                        )
                    } else {
                        format!(
                            "$|\\psi(y, z)|^2$ from $z = {}$ to ${}$ µm, the input at $y = {}$ µm",
                            sig(za, 3),
                            sig(zb, 3),
                            sig(y_in, 3)
                        )
                    },
                    raster: Raster {
                        nx,
                        ny,
                        x0: za,
                        x1: zb,
                        y0,
                        y1,
                        values: values
                            .iter()
                            .map(|v| ((v / top) * 1e3).round() as f32 / 1e3)
                            .collect(),
                    },
                    outline,
                }
            })
            .collect(),
        x_label: "along the section z".to_owned(),
        x_length: true,
        y_label: "across it y".to_owned(),
        y_log: false,
        y_range: None,
        series: Vec::new(),
        marker: None,
        figures: vec![
            figure(
                "$L_\\pi$",
                lpi,
                4,
                "µm",
                "π/(β₀ − β₁), from the exact modes",
            ),
            figure("Guided modes", c.len() as f64, 1, "", ""),
            figure(
                "Guided share $\\sum c_\\nu^2$",
                guided,
                5,
                "",
                "the rest is radiated at the junction",
            ),
            figure(
                "Odd modes' share",
                odd,
                2,
                "",
                "zero for a centred input, by symmetry",
            ),
            figure(
                "Share of modes 2, 5, 8, …",
                third,
                2,
                "",
                "small for an input at ±W/6",
            ),
        ],
        note: format!(
            "2D: the guided modes of a {} µm silicon section (220 nm of Li's silicon in Malitson's \
             silica, by the effective index method), fed by a 0.8 µm guide, at 1.55 µm; exact \
             slab modes, no grid; the field normal to the plane, its square scaled to its \
             brightest. This input: {mechanism}.",
            sig(width, 3)
        ),
    })
}

/// Each mode's phase lag behind the fundamental over 3L_π, (β₀ − β_ν) 3L_π/π, against the
/// paraxial ν(ν + 2) of Soldano and Pennings's Eq. 7, for a section `width` µm wide at `l` µm.
pub fn modes(width: f64, l: f64) -> photonoxide::Result<ChartData> {
    let lam = Wavelength::um(l)?;
    let access = (width / 4.0).min(0.5);
    let m = Mmi::new(MmiKind::OneByTwo, film(220.0), width, access, lam)?;
    let b = m.analysis(lam, width)?.propagation_constants();
    let lpi = PI / (b[0] - b[1]);
    let exact: Vec<[f64; 2]> = b
        .iter()
        .enumerate()
        .map(|(v, beta)| [v as f64, (b[0] - beta) * 3.0 * lpi / PI])
        .collect();
    let last = (b.len() - 1) as f64;
    let parabola: Vec<[f64; 2]> = (0..=100)
        .map(|k| {
            let v = last * f64::from(k) / 100.0;
            [v, v * (v + 2.0)]
        })
        .collect();
    let (nr, nc) = indices(220.0, l)?;
    let we = effective_width(width, l, nr, nc);
    let eq6 = 4.0 * nr * we * we / (3.0 * l);
    let error = exact.last().map_or(0.0, |p| p[1] - last * (last + 2.0));
    let four = exact.get(4).map_or(0.0, |p| (p[1] - 24.0) / 8.0);
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "mode number ν".to_owned(),
        x_length: false,
        y_label: "(β₀ − β_ν) · 3L_π / π".to_owned(),
        y_log: false,
        y_range: None,
        series: vec![
            curve("exact slab modes", exact, false, 0),
            curve("ν(ν + 2), Soldano and Pennings's Eq. 7", parabola, true, 1),
        ],
        marker: None,
        figures: vec![
            figure("Guided modes $m$", b.len() as f64, 1, "", ""),
            figure("$L_\\pi$, exact", lpi, 4, "µm", "π/(β₀ − β₁)"),
            figure(
                "$L_\\pi$, Eq. 6",
                eq6,
                4,
                "µm",
                &format!("4n_rW_e²/3λ: {} % of the exact", sig(100.0 * eq6 / lpi, 4)),
            ),
            figure("$W_e$, Eq. 4", we, 4, "µm", "the width the modes see"),
            figure("$n_r$", nr, 5, "", "the 220 nm film's TE index"),
            figure(
                "Last mode's phase error at $3L_\\pi$",
                error,
                3,
                "π",
                "its lag less ν(ν + 2)π",
            ),
            figure(
                "Mode 4's phase error at $3L_\\pi/8$",
                four,
                3,
                "π",
                "at the 1 × 2's image; 0 where the section guides no mode 4",
            ),
        ],
        note: format!(
            "2D: a {} µm silicon section's guided modes (220 nm of Li's silicon in Malitson's \
             silica, by the effective index method) at {} µm, exact slab modes, no grid. Where \
             the solid line leaves the dashed one, a mode's phase at an image misses the value \
             the self-imaging lengths assume.",
            sig(width, 3),
            sig(l, 4)
        ),
    })
}

/// The outputs' power from the first input against the section's length, for an MMI of `kind`
/// `width` µm wide with access guides `access` µm wide, at 1.55 µm.
pub fn length(kind: f64, width: f64, access: f64) -> photonoxide::Result<ChartData> {
    let kind = kind_of(kind)?;
    let l = Wavelength::um(DESIGN)?;
    let m = mmi(kind, width, access, 220.0)?;
    let a = m.analysis(l, width)?;
    let lpi = beat(&a);
    let design = design_length(&m);
    let (_, names) = outputs(kind);
    let n = 400;
    let lengths: Vec<f64> = (0..=n)
        .map(|k| 1.25 * lpi * f64::from(k) / f64::from(n))
        .collect();
    let mut upper = Vec::with_capacity(lengths.len());
    let mut lower = Vec::with_capacity(lengths.len());
    let mut both = Vec::with_capacity(lengths.len());
    for &z in &lengths {
        let (pa, pb, _) = split(&a, kind, z);
        upper.push([z, pa]);
        lower.push([z, pb]);
        both.push([z, pa + pb]);
    }
    let best = best(&a, kind, design);
    let (pa, pb, phase) = split(&a, kind, design);
    let (qa, qb, _) = split(&a, kind, best);
    let image = match kind {
        MmiKind::OneByTwo => "$3L_\\pi/8$",
        MmiKind::TwoByTwo => "$L_\\pi/2$",
    };
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "section length L".to_owned(),
        x_length: true,
        y_label: "power from the input".to_owned(),
        y_log: false,
        y_range: Some([0.0, 1.05]),
        series: vec![
            curve(names[0], upper, false, 0),
            curve(names[1], lower, kind == MmiKind::OneByTwo, 1),
            curve("both outputs", both, false, 2),
        ],
        marker: Some(design),
        figures: vec![
            figure(
                "$L_\\pi$",
                lpi,
                4,
                "µm",
                "π/(β₀ − β₁), from the exact modes",
            ),
            figure(
                &format!("Design length, {image}"),
                design,
                4,
                "µm",
                "the marker",
            ),
            figure(
                "Excess loss there",
                excess_db(pa, pb),
                3,
                "dB",
                &format!("{} and {} into the outputs", sig(pa, 4), sig(pb, 4)),
            ),
            figure("Imbalance there", imbalance_db(pa, pb), 3, "dB", ""),
            figure(
                "Phase difference there",
                phase,
                3,
                "°",
                "between the outputs' fields",
            ),
            figure(
                "Best length",
                best,
                4,
                "µm",
                &format!("{} % of the design's", sig(100.0 * best / design, 4)),
            ),
            figure(
                "Excess loss at the best",
                excess_db(qa, qb),
                3,
                "dB",
                &format!("imbalance {} dB", sig(imbalance_db(qa, qb), 2)),
            ),
        ],
        note: format!(
            "2D: the {} guided-mode analysis of a {} µm silicon section with {} µm access guides \
             (220 nm of Li's silicon in Malitson's silica, by the effective index method) at \
             1.55 µm; exact slab modes, no grid; radiation and reflections at the faces left out.",
            if kind == MmiKind::OneByTwo {
                "1 × 2"
            } else {
                "2 × 2"
            },
            sig(width, 3),
            sig(access, 3)
        ),
    })
}

/// The share of light into the outputs, the excess loss and the imbalance, at each
/// `(x, (pa, pb))`; and the band around `centre` where the excess loss and the imbalance both
/// stay under 1 dB: (from, to, whether it reaches an end of the sweep).
fn band(points: &[(f64, (f64, f64))], centre: f64) -> Option<(f64, f64, bool)> {
    let ok = |(pa, pb): (f64, f64)| excess_db(pa, pb) < 1.0 && imbalance_db(pa, pb).abs() < 1.0;
    let k = points
        .iter()
        .enumerate()
        .min_by(|a, b| (a.1.0 - centre).abs().total_cmp(&(b.1.0 - centre).abs()))?
        .0;
    if !ok(points[k].1) {
        return None;
    }
    let mut lo = k;
    while lo > 0 && ok(points[lo - 1].1) {
        lo -= 1;
    }
    let mut hi = k;
    while hi + 1 < points.len() && ok(points[hi + 1].1) {
        hi += 1;
    }
    Some((
        points[lo].0,
        points[hi].0,
        lo == 0 || hi + 1 == points.len(),
    ))
}

/// The outputs of an MMI of `kind`, `width` µm wide with `access` µm guides, cut at its best
/// length at 1.55 µm, from 1.40 to 1.70 µm.
pub fn spectrum(kind: f64, width: f64, access: f64) -> photonoxide::Result<ChartData> {
    let kind = kind_of(kind)?;
    let m = mmi(kind, width, access, 220.0)?;
    let design = best(
        &m.analysis(Wavelength::um(DESIGN)?, width)?,
        kind,
        design_length(&m),
    );
    let (_, names) = outputs(kind);
    let mut points = Vec::new();
    for k in 0..=120 {
        let l = 1.40 + 0.0025 * f64::from(k);
        let a = m.analysis(Wavelength::um(l)?, width)?;
        let (pa, pb, _) = split(&a, kind, design);
        points.push((l, (pa, pb)));
    }
    let at = |l: f64| -> photonoxide::Result<(f64, f64, f64)> {
        let a = m.analysis(Wavelength::um(l)?, width)?;
        Ok(split(&a, kind, design))
    };
    let mut figures = vec![figure(
        "Length",
        design,
        4,
        "µm",
        "its best at 1.55 µm, where the weaker output takes the most",
    )];
    for l in [1.50, 1.55, 1.60] {
        let (pa, pb, phase) = at(l)?;
        figures.push(figure(
            &format!("At {} µm: excess loss", sig(l, 3)),
            excess_db(pa, pb),
            3,
            "dB",
            &format!(
                "imbalance {} dB, phase difference {}°",
                sig(imbalance_db(pa, pb), 2),
                sig(phase, 3)
            ),
        ));
    }
    figures.push(match band(&points, DESIGN) {
        Some((lo, hi, end)) => figure(
            "Band under 1 dB",
            1e3 * (hi - lo),
            3,
            "nm",
            &format!(
                "excess loss and imbalance both under 1 dB from {} to {} µm{}",
                sig(lo, 4),
                sig(hi, 4),
                if end { ", to the sweep's end" } else { "" }
            ),
        ),
        None => figure(
            "Band under 1 dB",
            0.0,
            1,
            "nm",
            "not even at 1.55 µm: lengthen or widen it",
        ),
    });
    let series = vec![
        curve(
            names[0],
            points.iter().map(|(l, p)| [*l, p.0]).collect(),
            false,
            0,
        ),
        curve(
            names[1],
            points.iter().map(|(l, p)| [*l, p.1]).collect(),
            kind == MmiKind::OneByTwo,
            1,
        ),
        curve(
            "both outputs",
            points.iter().map(|(l, p)| [*l, p.0 + p.1]).collect(),
            false,
            2,
        ),
    ];
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "wavelength λ (µm)".to_owned(),
        x_length: false,
        y_label: "power from the input".to_owned(),
        y_log: false,
        y_range: Some([0.0, 1.05]),
        series,
        marker: Some(DESIGN),
        figures,
        note: format!(
            "2D: the guided-mode analysis of a {} µm silicon section with {} µm access guides, \
             {} µm long, the materials' and the film's dispersion included (Li's silicon, \
             Malitson's silica, by the effective index method); exact slab modes, no grid.",
            sig(width, 3),
            sig(access, 3),
            sig(design, 4)
        ),
    })
}

/// The access guides' centres: the MMI's ports at `width`, which stay put when the drawn
/// widths change.
fn centres(kind: MmiKind, width: f64) -> Vec<(f64, bool)> {
    match kind {
        MmiKind::OneByTwo => vec![(0.0, true), (width / 4.0, false), (-width / 4.0, false)],
        MmiKind::TwoByTwo => vec![
            (-width / 6.0, true),
            (width / 6.0, true),
            (width / 6.0, false),
            (-width / 6.0, false),
        ],
    }
}

/// The outputs of an MMI of `kind`, `width` µm wide with `access` µm guides, designed on 220 nm
/// at 1.55 µm, when every drawn width comes out Δw wider (−250 to 250 nm, the guides' centres in
/// place) on a film `thickness_nm` thick.
pub fn tolerance(
    kind: f64,
    width: f64,
    access: f64,
    thickness_nm: f64,
) -> photonoxide::Result<ChartData> {
    let kind = kind_of(kind)?;
    let l = Wavelength::um(DESIGN)?;
    let nominal_mmi = mmi(kind, width, access, 220.0)?;
    let design = best(
        &nominal_mmi.analysis(l, width)?,
        kind,
        design_length(&nominal_mmi),
    );
    let (_, names) = outputs(kind);
    let place = centres(kind, width);
    let at = |dw: f64| -> photonoxide::Result<(f64, f64)> {
        let (w, a) = (width + dw * 1e-3, access + dw * 1e-3);
        let m = mmi(kind, w, a, thickness_nm)?;
        let (pa, pb, _) = split(&m.analysis_at(l, w, &place)?, kind, design);
        Ok((pa, pb))
    };
    let mut points = Vec::new();
    for k in -50..=50 {
        let dw = 5.0 * f64::from(k);
        points.push((dw, at(dw)?));
    }
    let (p0, q0) = at(0.0)?;
    let nominal = excess_db(p0, q0);
    // the widths within 0.5 dB of the excess loss with none
    let within = |p: &(f64, (f64, f64))| excess_db(p.1.0, p.1.1) < nominal + 0.5;
    let k0 = points.len() / 2;
    let mut lo = k0;
    while lo > 0 && within(&points[lo - 1]) {
        lo -= 1;
    }
    let mut hi = k0;
    while hi + 1 < points.len() && within(&points[hi + 1]) {
        hi += 1;
    }
    let (nr, _) = indices(thickness_nm, DESIGN)?;
    // Soldano and Pennings's estimate: the length shift costing 0.5 dB, the image's Rayleigh
    // range π n_r w₀²/4λ (w₀ the input's full 1/e width, here the access guide's), and the width
    // change that moves the image so far, W δL/2L (their Eq. 6)
    let shift = PI * nr * access * access / (4.0 * DESIGN);
    let estimate = 1e3 * width * shift / (2.0 * design);
    let mut figures = vec![
        figure(
            "Length",
            design,
            4,
            "µm",
            "its best on 220 nm at 1.55 µm, where the weaker output takes the most",
        ),
        figure(
            "Excess loss with none",
            nominal,
            3,
            "dB",
            &format!("imbalance {} dB", sig(imbalance_db(p0, q0), 2)),
        ),
        figure(
            "Within 0.5 dB of it",
            points[hi].0 - points[lo].0,
            3,
            "nm",
            &format!(
                "Δw from {} to {} nm{}",
                sig(points[lo].0, 3),
                sig(points[hi].0, 3),
                if lo == 0 || hi + 1 == points.len() {
                    ", to the chart's end"
                } else {
                    ""
                }
            ),
        ),
        figure(
            "Soldano and Pennings's estimate",
            2.0 * estimate,
            3,
            "nm",
            &format!(
                "±W δL/2L, δL = πn_r w₀²/4λ = {} µm with w₀ the access width",
                sig(shift, 3)
            ),
        ),
    ];
    for dw in [-20.0, 20.0] {
        let (pa, pb) = at(dw)?;
        figures.push(figure(
            &format!("At Δw = {} nm: excess loss", sig(dw, 2)),
            excess_db(pa, pb),
            3,
            "dB",
            &format!("imbalance {} dB", sig(imbalance_db(pa, pb), 2)),
        ));
    }
    let series = vec![
        curve(
            names[0],
            points.iter().map(|(x, p)| [*x, p.0]).collect(),
            false,
            0,
        ),
        curve(
            names[1],
            points.iter().map(|(x, p)| [*x, p.1]).collect(),
            kind == MmiKind::OneByTwo,
            1,
        ),
        curve(
            "both outputs",
            points.iter().map(|(x, p)| [*x, p.0 + p.1]).collect(),
            false,
            2,
        ),
    ];
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "width error Δw (nm)".to_owned(),
        x_length: false,
        y_label: "power from the input".to_owned(),
        y_log: false,
        y_range: Some([0.0, 1.05]),
        series,
        marker: Some(0.0),
        figures,
        note: format!(
            "2D: the guided-mode analysis at 1.55 µm of a {} µm section with {} µm access guides, \
             {} µm long (its best length on 220 nm), every drawn width Δw wider and the guides' centres \
             in place, on a film {} nm thick (Li's silicon in Malitson's silica, by the effective \
             index method); exact slab modes, no grid.",
            sig(width, 3),
            sig(access, 3),
            sig(design, 4),
            sig(thickness_nm, 3)
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use photonoxide::portable;

    fn fig(data: &ChartData, label: &str) -> f64 {
        data.figures
            .iter()
            .find(|f| f.label.starts_with(label))
            .unwrap_or_else(|| panic!("no figure {label}"))
            .value
            .unwrap()
    }

    /// The length chart is the MMI's S-matrix, as the component computes it, at every length;
    /// the 1 × 2 splits evenly and in phase, the 2 × 2 a quarter turn apart.
    #[test]
    fn the_length_chart_is_the_components_s_matrix() {
        use photonoxide::circuit::Component;
        for (k, kind) in [(1.0, MmiKind::OneByTwo), (2.0, MmiKind::TwoByTwo)] {
            let data = length(k, 3.0, 0.8).unwrap();
            let m = mmi(kind, 3.0, 0.8, 220.0).unwrap();
            let ((p, q), _) = outputs(kind);
            for (a, b) in data.series[0]
                .points
                .iter()
                .zip(&data.series[1].points)
                .step_by(37)
            {
                let s = m
                    .s_matrix(Wavelength::um(1.55).unwrap(), &[a[0], 3.0])
                    .unwrap();
                assert_eq!(a[1], s.power(p, 0));
                assert_eq!(b[1], s.power(q, 0));
            }
            assert_eq!(data.marker, Some(m.defaults()[0]));
            let phase = fig(&data, "Phase difference");
            match kind {
                MmiKind::OneByTwo => {
                    assert!(phase.abs() < 1e-6, "{phase}");
                    assert!(fig(&data, "Imbalance").abs() < 1e-9);
                }
                MmiKind::TwoByTwo => assert!((phase.abs() - 90.0).abs() < 15.0, "{phase}"),
            }
            // the examples' numbers: mmi_soldano's 3 µm sections with 0.8 µm guides
            let loss = fig(&data, "Excess loss there");
            let printed = if kind == MmiKind::OneByTwo {
                0.429
            } else {
                0.707
            };
            assert!((loss - printed).abs() < 5e-4, "{kind:?}: {loss}");
            // the best length gives the weaker output at least what the design length does
            let a = m.analysis(Wavelength::um(1.55).unwrap(), 3.0).unwrap();
            let design = m.defaults()[0];
            let (pa, pb, _) = split(&a, kind, best(&a, kind, design));
            let (qa, qb, _) = split(&a, kind, design);
            assert!(pa.min(pb) >= qa.min(qb) - 1e-12, "{kind:?}");
        }
    }

    /// The field chart's picture is the guided modes' sum; a centred input excites no odd mode,
    /// one at W/6 almost none of modes 2, 5, 8.
    #[test]
    fn the_field_is_the_guided_modes_and_the_input_picks_the_mechanism() {
        let data = field(3.0, 0.0, 1.0).unwrap();
        assert!(fig(&data, "Odd modes") < 1e-9);
        assert!(data.note.contains("symmetric"), "{}", data.note);
        let r = &data.maps[0].raster;
        assert!(r.values.iter().all(|v| (0.0..=1.0).contains(v)));
        assert_eq!(r.values.iter().copied().fold(0.0, f32::max), 1.0);
        let paired = field(3.0, -1.0 / 6.0, 1.0).unwrap();
        assert!(
            fig(&paired, "Share of modes 2") < 0.01,
            "{:?}",
            paired.figures
        );
        assert!(paired.note.contains("paired"), "{}", paired.note);
        let general = field(3.0, -0.3, 1.0).unwrap();
        assert!(general.note.contains("general"), "{}", general.note);
        assert!(field(3.0, 0.45, 1.0).is_err());
        // a long section in rows that join end to end, on one colour scale
        let long = field(2.5, -0.3, 3.2).unwrap();
        assert_eq!(long.maps.len(), 2);
        assert_eq!(long.maps[0].raster.x1, long.maps[1].raster.x0);
        let brightest = long
            .maps
            .iter()
            .flat_map(|m| m.raster.values.iter().copied())
            .fold(0.0, f32::max);
        assert_eq!(brightest, 1.0);
        // the beat length is the component's
        let m = mmi(MmiKind::OneByTwo, 3.0, 0.8, 220.0).unwrap();
        let lpi = m.beat_length(Wavelength::um(1.55).unwrap(), 3.0).unwrap();
        assert_eq!(fig(&data, "$L_\\pi$"), lpi);
    }

    /// The modes' phases against Eq. 7, and Eq. 6 within 1 % of the exact beat length at 3 µm, as
    /// the validation case components/mmi-beat-length-soldano has it (22.8115 against 22.6035).
    #[test]
    fn the_modes_chart_is_soldano_and_penningss_parabola() {
        let data = modes(3.0, 1.55).unwrap();
        let exact = fig(&data, "$L_\\pi$, exact");
        let eq6 = fig(&data, "$L_\\pi$, Eq. 6");
        assert!((exact - 22.6035).abs() < 1e-3, "{exact}");
        assert!((eq6 - 22.8115).abs() < 1e-3, "{eq6}");
        // the first two points are 0 and 3 by the definition of L_π
        let p = &data.series[0].points;
        assert_eq!(p[0][1], 0.0);
        assert!((p[1][1] - 3.0).abs() < 1e-12);
        // the higher modes lag more than the parabola: the exact β falls faster than quadratic
        assert!(fig(&data, "Last mode") > 0.0);
        // wider sections are more paraxial
        let wide = modes(12.0, 1.55).unwrap();
        let ratio = |d: &ChartData| fig(d, "$L_\\pi$, Eq. 6") / fig(d, "$L_\\pi$, exact");
        assert!((ratio(&wide) - 1.0).abs() < (ratio(&data) - 1.0).abs());
    }

    /// The spectrum at 1.55 µm is the length chart's design point; the band holds 1.55 µm.
    #[test]
    fn the_spectrum_meets_the_design_at_its_wavelength() {
        for k in [1.0, 2.0] {
            let s = spectrum(k, 3.0, 0.8).unwrap();
            let d = length(k, 3.0, 0.8).unwrap();
            let at = s.series[2]
                .points
                .iter()
                .find(|p| (p[0] - 1.55).abs() < 1e-9)
                .unwrap()[1];
            let loss = fig(&d, "Excess loss at the best");
            assert!(
                (-10.0 * portable::log10(at) - loss).abs() < 1e-9,
                "kind {k}"
            );
            assert!(fig(&s, "Band under 1 dB") > 0.0, "kind {k}");
        }
    }

    /// With no width error the tolerance chart is the design; widening the section moves the
    /// image and costs loss either way.
    #[test]
    fn the_tolerance_chart_starts_at_the_design() {
        let t = tolerance(1.0, 3.0, 0.8, 220.0).unwrap();
        let d = length(1.0, 3.0, 0.8).unwrap();
        assert!(
            (fig(&t, "Excess loss with none") - fig(&d, "Excess loss at the best")).abs() < 1e-9
        );
        let both = &t.series[2].points;
        let mid = both[both.len() / 2][1];
        assert!(both[0][1] < mid && both[both.len() - 1][1] < mid);
        assert!(fig(&t, "Within 0.5 dB") > 0.0);
        // a thinner film lowers n_r and shortens the image: the loss at Δw = 0 grows
        let thin = tolerance(1.0, 3.0, 0.8, 200.0).unwrap();
        assert!(fig(&thin, "Excess loss with none") > fig(&t, "Excess loss with none"));
    }
}
