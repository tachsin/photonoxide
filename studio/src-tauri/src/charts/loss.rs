//! The charts of the lesson on loss in silicon waveguides: the loss rough sidewalls radiate from a
//! slab's TE₀ mode, by the volume current on the exact mode (`mode::roughness::slab_loss`) beside
//! Payne and Lacey's closed form (`mode::roughness::payne_lacey`), against the core's thickness
//! and against the roughness's correlation length; and a cutback measurement of a waveguide's
//! loss, its transmissions from `circuit::components::Waveguide` and its straight line fitted.

use photonoxide::circuit::Component;
use photonoxide::circuit::components::{Dispersion, Waveguide};
use photonoxide::mode::roughness::{Roughness, db_per_cm, payne_lacey, slab_loss};
use photonoxide::portable;
use photonoxide::units::{Length, Wavelength};

use super::{ChartData, ChartSpec, Curve, figure, invalid, param, sig};

/// The palette's grey, for reference lines.
const GREY: usize = 7;
/// The cladding: silica, as roughness/payne-lacey-slab takes it.
const CLADDING: f64 = 1.45;
/// The wavelength the roughness charts take, µm.
const WAVELENGTH: f64 = 1.55;
/// The cutback chart's guide: n_eff and n_g of a 500 × 220 nm silicon strip at 1.55 µm.
const CUTBACK_NEFF: f64 = 2.4;
const CUTBACK_NG: f64 = 4.2;
/// The cutback's lengths: this many, evenly spaced up to the longest.
const CUTBACK_LENGTHS: usize = 6;
/// The measurement's scatter, each length's share of `scatter`: a fixed pattern, so the chart
/// is the same every time, with no trend of its own.
const SCATTER: [f64; CUTBACK_LENGTHS] = [0.8, -1.0, 0.3, 0.9, -0.6, -0.4];

fn curve(label: &str, points: Vec<[f64; 2]>, dashed: bool, colour: usize) -> Curve {
    Curve {
        label: label.to_owned(),
        points,
        dashed,
        colour: Some(colour),
    }
}

/// `n` points from `a` to `b`, both included, evenly on a logarithmic scale.
fn log_span(a: f64, b: f64, n: usize) -> Vec<f64> {
    (0..n)
        .map(|k| a * portable::powf(b / a, k as f64 / (n - 1) as f64))
        .collect()
}

/// The lesson's charts.
pub fn specs() -> Vec<ChartSpec> {
    let sigma = param(
        "sigma",
        "Roughness $\\sigma$",
        "nm",
        (0.1, 10.0, 2.0),
        0.1,
        false,
        "The walls' rms deviation from straight.",
    );
    let core = param(
        "core",
        "Core $n_1$",
        "",
        (1.5, 3.6, 3.5),
        0.01,
        false,
        "The core's index: 3.5 for silicon, 2.0 for silicon nitride, 1.5 for silicon oxynitride; the cladding is silica, 1.45.",
    );
    vec![
        ChartSpec {
            id: "loss-roughness",
            title: "Roughness loss against the core's thickness",
            about: "A slab of index n₁ in silica, both walls rough with σ and L_c: its TE₀ mode's loss at 1.55 µm against the slab's thickness, by the volume current on the exact mode and by Payne and Lacey's closed form. Thin cores push the field onto the walls; very thin ones let it spill into the cladding, where the walls no longer see it.",
            computed_by: "mode::roughness::slab_loss (the volume current on the exact mode::slab mode, by quadrature) and mode::roughness::payne_lacey (their Eqs. 8–12), at 160 thicknesses",
            params: vec![
                sigma.clone(),
                param(
                    "correlation",
                    "Correlation length $L_c$",
                    "nm",
                    (5.0, 2000.0, 50.0),
                    1.0,
                    true,
                    "How far along the wall its deviation stays the same: the autocorrelation falls as e^(−|u|/L_c).",
                ),
                core.clone(),
                param(
                    "thickness",
                    "Thickness $2d$",
                    "nm",
                    (40.0, 1000.0, 220.0),
                    1.0,
                    false,
                    "The slab's thickness, between its rough walls: the figures are at it.",
                ),
            ],
        },
        ChartSpec {
            id: "loss-correlation",
            title: "Roughness loss against the correlation length",
            about: "The same slab at a fixed thickness: its loss against the roughness's correlation length L_c. Short correlations scatter at every angle, long ones barely at all; the loss is largest between, where the roughness's spectrum is strongest at the spatial frequencies that send light into the cladding.",
            computed_by: "mode::roughness::slab_loss and mode::roughness::payne_lacey at 120 correlation lengths from 5 nm to 5 µm",
            params: vec![
                sigma,
                core,
                param(
                    "thickness",
                    "Thickness $2d$",
                    "nm",
                    (40.0, 1000.0, 220.0),
                    1.0,
                    false,
                    "The slab's thickness, between its rough walls.",
                ),
                param(
                    "correlation",
                    "Correlation length $L_c$",
                    "nm",
                    (5.0, 2000.0, 50.0),
                    1.0,
                    true,
                    "The correlation length the figures are at.",
                ),
            ],
        },
        ChartSpec {
            id: "loss-cutback",
            title: "A cutback measurement",
            about: "Guides of six lengths on one chip, each measured from fibre to fibre: the transmission in dB falls on a straight line whose slope is the loss per length and whose intercept is the two facets' coupling. Each measurement is off by a fixed pattern of scatter, as facets coupled anew each time are; the fitted slope carries its uncertainty.",
            computed_by: "circuit::components::Waveguide::s_matrix (e^(iγL) with the loss) at each length, minus the facets' coupling; the straight line by least squares, its slope's standard error from the residuals",
            params: vec![
                param(
                    "loss",
                    "Loss $\\alpha$",
                    "dB/cm",
                    (0.1, 20.0, 2.0),
                    0.1,
                    false,
                    "The guide's propagation loss, the slope the measurement looks for.",
                ),
                param(
                    "facet",
                    "Coupling per facet",
                    "dB",
                    (0.0, 10.0, 4.0),
                    0.1,
                    false,
                    "What each facet's coupling to a fibre loses, the same for every guide.",
                ),
                param(
                    "longest",
                    "Longest guide (cm)",
                    "",
                    (0.2, 10.0, 2.0),
                    0.1,
                    false,
                    "The longest guide, cm; the others are 1/6, 2/6, … of it.",
                ),
                param(
                    "scatter",
                    "Scatter",
                    "dB",
                    (0.0, 2.0, 0.3),
                    0.05,
                    false,
                    "How far a measurement strays from the line, as recoupling a fibre makes it: each length off by a fixed share of it.",
                ),
            ],
        },
    ]
}

/// The lesson's chart `id`, if it is one.
pub fn evaluate(id: &str, at: impl Fn(&str) -> f64) -> Option<photonoxide::Result<ChartData>> {
    Some(match id {
        "loss-roughness" => roughness(at("sigma"), at("correlation"), at("core"), at("thickness")),
        "loss-correlation" => {
            correlation(at("sigma"), at("core"), at("thickness"), at("correlation"))
        }
        "loss-cutback" => cutback(at("loss"), at("facet"), at("longest"), at("scatter")),
        _ => return None,
    })
}

/// A slab's roughness loss in dB/cm, both walls rough: by the volume current on the exact mode,
/// and by Payne and Lacey's Eq. 10 as printed (its 1/n₁ the weakly guiding slab's k₀/β).
fn slab_db(
    core: f64,
    thickness_nm: f64,
    sigma_nm: f64,
    correlation_nm: f64,
) -> photonoxide::Result<(f64, f64, f64, f64)> {
    let roughness = Roughness::exponential(Length::nm(sigma_nm), Length::nm(correlation_nm))?;
    let t = Length::nm(thickness_nm);
    let w = Wavelength::um(WAVELENGTH)?;
    let exact = slab_loss(core, CLADDING, t, w, &roughness)?;
    let closed = payne_lacey(core, CLADDING, t, w, &roughness)?;
    Ok((
        exact.db_per_cm(),
        db_per_cm(closed.eq10),
        exact.effective_index,
        exact.field_at_wall,
    ))
}

/// Roughness loss against the slab's thickness, 40 nm to 1 µm, the figures at `thickness` nm.
pub fn roughness(
    sigma: f64,
    correlation: f64,
    core: f64,
    thickness: f64,
) -> photonoxide::Result<ChartData> {
    if core <= CLADDING {
        return Err(invalid(
            "core",
            "the core's index must exceed the cladding's",
        ));
    }
    let (mut exact, mut closed) = (Vec::new(), Vec::new());
    for t in log_span(40.0, 1000.0, 160) {
        let (e, c, _, _) = slab_db(core, t, sigma, correlation)?;
        exact.push([t * 1e-3, e]);
        closed.push([t * 1e-3, c]);
    }
    let (e, c, neff, wall) = slab_db(core, thickness, sigma, correlation)?;
    let peak = exact
        .iter()
        .copied()
        .fold([0.0, 0.0], |a, p| if p[1] > a[1] { p } else { a });
    let top = exact
        .iter()
        .chain(&closed)
        .map(|p| p[1])
        .fold(0.0, f64::max);
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "thickness 2d".to_owned(),
        x_length: true,
        y_label: "loss (dB/cm)".to_owned(),
        y_log: false,
        y_range: Some([0.0, 1.1 * top]),
        series: vec![
            curve("volume current, exact mode", exact, false, 0),
            curve("Payne and Lacey, Eq. 10", closed, true, 1),
        ],
        marker: Some(thickness * 1e-3),
        figures: vec![
            figure(
                &format!("Loss at {} nm", sig(thickness, 3)),
                e,
                3,
                "dB/cm",
                "both walls, by the volume current on the exact mode",
            ),
            figure(
                "Payne and Lacey there",
                c,
                3,
                "dB/cm",
                "their Eq. 10 as printed: its 1/n₁ is k₀/β, so it is the exact loss times β/(n₁k₀)",
            ),
            figure(
                "Per nm² of roughness",
                e / (sigma * sigma),
                3,
                "dB/cm/nm²",
                "the loss grows as σ², so this holds for any σ",
            ),
            figure(
                "$n_\\text{eff}$ there",
                neff,
                4,
                "",
                "the TE₀ mode's effective index",
            ),
            figure(
                "$\\varphi^2(d)$ at a wall",
                wall,
                3,
                "1/µm",
                "the field at a wall squared, the field normalized to ∫φ² = 1: what the wall scatters",
            ),
            figure(
                "Lossiest thickness",
                peak[0] * 1e3,
                3,
                "nm",
                &format!("{} dB/cm there", sig(peak[1], 3)),
            ),
        ],
        note: format!(
            "Exact slab modes, no grid: TE₀ of a symmetric slab of index {} in {CLADDING} at \
             {WAVELENGTH} µm, σ = {} nm, L_c = {} nm, both walls rough and uncorrelated; each \
             loss by Gauss–Legendre sums on the exact field (slab_loss) and in closed form \
             (payne_lacey). First order in σ.",
            sig(core, 3),
            sig(sigma, 3),
            sig(correlation, 3)
        ),
    })
}

/// Roughness loss against the correlation length, 5 nm to 5 µm, at `thickness` nm; the figures
/// at `correlation` nm.
pub fn correlation(
    sigma: f64,
    core: f64,
    thickness: f64,
    correlation: f64,
) -> photonoxide::Result<ChartData> {
    if core <= CLADDING {
        return Err(invalid(
            "core",
            "the core's index must exceed the cladding's",
        ));
    }
    let (mut exact, mut closed) = (Vec::new(), Vec::new());
    for lc in log_span(5.0, 5000.0, 120) {
        let (e, c, _, _) = slab_db(core, thickness, sigma, lc)?;
        exact.push([lc * 1e-3, e]);
        closed.push([lc * 1e-3, c]);
    }
    let (e, _, neff, _) = slab_db(core, thickness, sigma, correlation)?;
    let peak = exact
        .iter()
        .copied()
        .fold([0.0, 0.0], |a, p| if p[1] > a[1] { p } else { a });
    // the spatial frequency that sends light along the cladding, β − n₂k₀: its inverse is the
    // correlation length whose spectrum still holds it
    let k0 = 2.0 * std::f64::consts::PI / WAVELENGTH;
    let grazing = 1.0 / ((neff - CLADDING) * k0);
    let top = exact
        .iter()
        .chain(&closed)
        .map(|p| p[1])
        .fold(0.0, f64::max);
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "correlation length L_c".to_owned(),
        x_length: true,
        y_label: "loss (dB/cm)".to_owned(),
        y_log: false,
        y_range: Some([0.0, 1.1 * top]),
        series: vec![
            curve("volume current, exact mode", exact, false, 0),
            curve("Payne and Lacey, Eq. 10", closed, true, 1),
        ],
        marker: Some(correlation * 1e-3),
        figures: vec![
            figure(
                &format!("Loss at $L_c$ = {} nm", sig(correlation, 3)),
                e,
                3,
                "dB/cm",
                "both walls, by the volume current on the exact mode",
            ),
            figure(
                "Lossiest $L_c$",
                peak[0] * 1e3,
                3,
                "nm",
                &format!("{} dB/cm there", sig(peak[1], 3)),
            ),
            figure(
                "$1/(\\beta - n_2 k_0)$",
                grazing * 1e3,
                3,
                "nm",
                "the length whose spatial frequency sends light along the cladding; the lossiest L_c is of its order",
            ),
        ],
        note: format!(
            "Exact slab modes, no grid: TE₀ of a symmetric slab {} nm thick of index {} in \
             {CLADDING} at {WAVELENGTH} µm, σ = {} nm, the correlation length on a logarithmic \
             scale from 5 nm to 5 µm. First order in σ.",
            sig(thickness, 3),
            sig(core, 3),
            sig(sigma, 3)
        ),
    })
}

/// A least-squares straight line through `points`: its slope, intercept and the slope's
/// standard error.
fn line(points: &[[f64; 2]]) -> (f64, f64, f64) {
    let n = points.len() as f64;
    let mx = points.iter().map(|p| p[0]).sum::<f64>() / n;
    let my = points.iter().map(|p| p[1]).sum::<f64>() / n;
    let sxx: f64 = points.iter().map(|p| portable::powi(p[0] - mx, 2)).sum();
    let sxy: f64 = points.iter().map(|p| (p[0] - mx) * (p[1] - my)).sum();
    let slope = sxy / sxx;
    let intercept = my - slope * mx;
    let residuals: f64 = points
        .iter()
        .map(|p| portable::powi(p[1] - intercept - slope * p[0], 2))
        .sum();
    let error = (residuals / (n - 2.0)).sqrt() / sxx.sqrt();
    (slope, intercept, error)
}

/// The fibre-to-fibre transmission, dB, of a guide `length_cm` long with `loss` dB/cm and two
/// facets of `facet` dB each.
fn transmission(
    wire: &Waveguide,
    length_cm: f64,
    loss: f64,
    facet: f64,
) -> photonoxide::Result<f64> {
    let s = wire.s_matrix(Wavelength::um(WAVELENGTH)?, &[length_cm * 1e4, loss])?;
    Ok(10.0 * portable::log10(s[(1, 0)].norm_sqr()) - 2.0 * facet)
}

/// A cutback: six guides up to `longest` cm, each measured off the line by its share of
/// `scatter` dB, and the line fitted.
pub fn cutback(
    loss: f64,
    facet: f64,
    longest: f64,
    scatter: f64,
) -> photonoxide::Result<ChartData> {
    let wire = Waveguide::new(
        Dispersion::new(Wavelength::um(WAVELENGTH)?, CUTBACK_NEFF, CUTBACK_NG).with_loss(loss),
    );
    let lengths: Vec<f64> = (1..=CUTBACK_LENGTHS)
        .map(|k| longest * k as f64 / CUTBACK_LENGTHS as f64)
        .collect();
    let mut measured = Vec::with_capacity(CUTBACK_LENGTHS);
    for (l, share) in lengths.iter().zip(SCATTER) {
        measured.push([*l, transmission(&wire, *l, loss, facet)? + share * scatter]);
    }
    let (slope, intercept, error) = line(&measured);
    let span: Vec<f64> = (0..=40).map(|k| longest * f64::from(k) / 40.0).collect();
    let mut truth = Vec::with_capacity(span.len());
    for &l in &span {
        truth.push([l, transmission(&wire, l, loss, facet)?]);
    }
    let fitted: Vec<[f64; 2]> = span.iter().map(|&l| [l, intercept + slope * l]).collect();
    // the intrinsic Q a ring of this guide would have: 2π n_g/(λα), α per µm of power
    let alpha = loss / db_per_cm(1.0);
    let q = 2.0 * std::f64::consts::PI * CUTBACK_NG / (WAVELENGTH * alpha);
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "guide length (cm)".to_owned(),
        x_length: false,
        y_label: "fibre to fibre (dB)".to_owned(),
        y_log: false,
        y_range: None,
        series: vec![
            curve("measured", measured, false, 0),
            curve("fitted line", fitted, true, 1),
            curve("the guide itself", truth, true, GREY),
        ],
        marker: None,
        figures: vec![
            figure(
                "Fitted loss",
                -slope,
                3,
                "dB/cm",
                &format!(
                    "± {} dB/cm, one standard error from {CUTBACK_LENGTHS} lengths",
                    sig(error, 2)
                ),
            ),
            figure(
                "Fitted coupling per facet",
                -intercept / 2.0,
                3,
                "dB",
                "half the line's intercept at zero length",
            ),
            figure(
                "The guide's loss",
                loss,
                3,
                "dB/cm",
                "what the measurement should find",
            ),
            figure(
                "Intrinsic $Q_i$ of a ring",
                q,
                3,
                "",
                &format!("2πn_g/(λα), n_g {CUTBACK_NG}: the same loss measured by a ring"),
            ),
        ],
        note: format!(
            "Closed forms, no grid: e^(iγL) of a guide of n_eff {CUTBACK_NEFF} and n_g \
             {CUTBACK_NG} at {WAVELENGTH} µm with its loss, minus two facets; the scatter a fixed \
             pattern ({}) times {} dB.",
            SCATTER.map(|s| sig(s, 2)).join(", "),
            sig(scatter, 2)
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fig(data: &ChartData, label: &str) -> f64 {
        data.figures
            .iter()
            .find(|f| f.label.starts_with(label))
            .unwrap_or_else(|| panic!("no figure {label}"))
            .value
            .unwrap()
    }

    /// The curves are the library's: slab_loss and payne_lacey at each thickness, and Eq. 10
    /// as printed is the exact loss times β/(n₁k₀) (roughness/payne-lacey-slab).
    #[test]
    fn the_roughness_curves_are_the_librarys() {
        let data = roughness(2.0, 50.0, 3.5, 220.0).unwrap();
        let walls = Roughness::exponential(Length::nm(2.0), Length::nm(50.0)).unwrap();
        let w = Wavelength::um(WAVELENGTH).unwrap();
        // each thickness as the chart makes it, in nm: Length::um(x) of its axis's µm value
        // can be another bit, and the library's loss follows it
        for ((exact, closed), nm) in data.series[0]
            .points
            .iter()
            .zip(&data.series[1].points)
            .zip(log_span(40.0, 1000.0, 160))
            .step_by(23)
        {
            assert_eq!(exact[0], nm * 1e-3);
            let t = Length::nm(nm);
            let e = slab_loss(3.5, CLADDING, t, w, &walls).unwrap();
            assert_eq!(exact[1], e.db_per_cm());
            let ratio = closed[1] / exact[1];
            let expected = e.effective_index / 3.5;
            assert!(
                (ratio - expected).abs() < 1e-9,
                "{} nm: {ratio} against {expected}",
                exact[0] * 1e3
            );
        }
        // the loss grows as σ²
        let twice = roughness_at(4.0);
        assert!((twice / fig(&data, "Loss at") - 4.0).abs() < 1e-12);
        // thin cores lose most, between the cutoff-free thin limit and the thick one
        let peak = fig(&data, "Lossiest thickness");
        assert!(peak > 40.0 && peak < 400.0, "{peak} nm");
        assert!(roughness(2.0, 50.0, 1.4, 220.0).is_err());
    }

    fn roughness_at(sigma: f64) -> f64 {
        fig(&roughness(sigma, 50.0, 3.5, 220.0).unwrap(), "Loss at")
    }

    /// The loss vanishes for very short and very long correlations and peaks between, of the
    /// order of 1/(β − n₂k₀).
    #[test]
    fn the_correlation_curve_peaks_between() {
        let data = correlation(2.0, 3.5, 220.0, 50.0).unwrap();
        let points = &data.series[0].points;
        let peak = fig(&data, "Lossiest");
        let top = points.iter().map(|p| p[1]).fold(0.0, f64::max);
        assert!(points.first().unwrap()[1] < 0.5 * top && points.last().unwrap()[1] < 0.5 * top);
        let grazing = fig(&data, "$1/(");
        assert!(
            peak / grazing > 0.2 && peak / grazing < 5.0,
            "{peak} against {grazing}"
        );
        assert_eq!(
            fig(&data, "Loss at"),
            fig(&roughness(2.0, 50.0, 3.5, 220.0).unwrap(), "Loss at")
        );
    }

    /// Without scatter the fit finds the loss and the facets exactly; with it, within a few
    /// standard errors.
    #[test]
    fn the_cutback_finds_the_loss() {
        let clean = cutback(2.5, 3.0, 2.0, 0.0).unwrap();
        assert!((fig(&clean, "Fitted loss") - 2.5).abs() < 1e-9);
        assert!((fig(&clean, "Fitted coupling") - 3.0).abs() < 1e-9);
        let noisy = cutback(2.5, 3.0, 2.0, 0.5).unwrap();
        let error = noisy.figures[0]
            .note
            .split_whitespace()
            .nth(1)
            .unwrap()
            .parse::<f64>()
            .unwrap();
        assert!(error > 0.0 && (fig(&noisy, "Fitted loss") - 2.5).abs() < 4.0 * error);
        // a ring's intrinsic Q: 2 dB/cm and n_g 4.2 give 3.697 × 10⁵
        let q = fig(&cutback(2.0, 3.0, 2.0, 0.0).unwrap(), "Intrinsic");
        assert!((q - 3.697e5).abs() < 0.001e5, "{q}");
    }
}
